use serde_json::json;

use crate::{
    AppState, Shared, ai,
    error::{AppResult, user, user_err},
    pipeline::{risks_json, simulation_json},
    portfolio, risk, simulate,
    solana::{
        LAMPORTS_PER_SOL, LookupTable, Message, Pubkey, TOKEN_PROGRAM, decode_transaction, from_base64, memo,
        parse_lookup_table, program_name, pseudo_random_key, set_compute_unit_price, system_transfer, to_base64,
        token_approve,
    },
    stream::Emitter,
    types::{
        InspectRequest, InspectionResult, RiskFlag, RiskLevel, SampleResponse, StageStatus, StreamEvent, TamperRequest,
        TamperResponse,
    },
};

async fn load_tables(st: &AppState, message: &Message) -> AppResult<Vec<LookupTable>> {
    if message.lookups.is_empty() {
        return Ok(Vec::new());
    }
    let keys: Vec<Pubkey> = message.lookups.iter().map(|lookup| lookup.table).collect();
    let accounts = st.rpc.accounts(&keys).await?;
    keys.iter()
        .zip(accounts)
        .map(|(key, account)| {
            account
                .and_then(|a| parse_lookup_table(*key, &a.owner, &a.data))
                .ok_or_else(|| user_err(format!("Lookup table {} used by this transaction no longer exists.", key.short())))
        })
        .collect()
}

fn decode(encoded: &str) -> AppResult<Message> {
    let bytes = from_base64(encoded).map_err(|_| user_err("Paste the transaction as base64 text."))?;
    if bytes.len() < 64 || bytes.len() > 4096 {
        return user("That doesn't look like a Solana transaction.");
    }
    decode_transaction(&bytes).map_err(|err| user_err(format!("Couldn't decode the transaction: {err}")))
}

pub async fn run(st: Shared, request: InspectRequest, em: Emitter) -> AppResult<()> {
    em.stages(&[("decode", "Decode"), ("simulate", "Simulate"), ("analyze", "Analyze")]).await;
    em.stage("decode", StageStatus::Active, None).await;
    let message = decode(&request.transaction)?;
    let tables = load_tables(&st, &message).await?;
    let keys = message.account_keys(&tables).map_err(|err| user_err(err.to_string()))?;
    let fee_payer = message.fee_payer().ok_or_else(|| user_err("The transaction has no fee payer."))?;
    let perspective = request
        .owner
        .as_deref()
        .and_then(Pubkey::parse)
        .unwrap_or(fee_payer);
    let signs = message
        .keys
        .iter()
        .take(message.header.required_signatures as usize)
        .any(|key| *key == perspective);
    let memos: Vec<String> = message
        .instructions
        .iter()
        .filter_map(|ix| {
            let program = keys.get(ix.program_index as usize)?.to_string();
            program_name(&program)
                .filter(|name| name.starts_with("Memo"))
                .map(|_| String::from_utf8_lossy(&ix.data).chars().take(280).collect())
        })
        .collect();
    em.stage(
        "decode",
        StageStatus::Done,
        Some(format!("{} instructions · {} accounts", message.instructions.len(), keys.len())),
    )
    .await;

    em.stage("simulate", StageStatus::Active, None).await;
    let outcome = simulate::run(&st, &message, &tables, &perspective).await?;
    let sim_status = if outcome.summary.ok { StageStatus::Done } else { StageStatus::Error };
    em.stage("simulate", sim_status, Some(format!("{} balance changes", outcome.summary.changes.len()))).await;

    if em.closed() {
        return Ok(());
    }
    em.stage("analyze", StageStatus::Active, None).await;
    let mut risks = risk::simulation_risks(&outcome.summary, true, outcome.pre_lamports as f64 / LAMPORTS_PER_SOL);
    if !signs {
        risks.push(RiskFlag::new(
            RiskLevel::Info,
            "not_signer",
            "Your wallet doesn't sign this transaction",
            "It can't move your funds directly, though it may touch accounts you use.",
        ));
    }
    if !memos.is_empty() {
        risks.push(RiskFlag::new(
            RiskLevel::Info,
            "memo_text",
            "Contains a text message",
            "Messages inside transactions are written by whoever built them. Scammers use them for fake reward claims.",
        ));
    }
    risk::sort(&mut risks);
    let floor = risk::verdict(&risks, outcome.summary.ok);
    let facts_json = json!({
        "kind": "inspection",
        "perspectiveWallet": perspective.short(),
        "walletSigns": signs,
        "instructionCount": message.instructions.len(),
        "untrustedMemoText": memos,
        "simulation": simulation_json(&outcome.summary),
        "riskFlags": risks_json(&risks),
        "ruleVerdict": floor,
    });
    let subject = format!("A transaction with {} instructions", message.instructions.len());
    let (explanation, warning) = ai::explain(&st, &facts_json, floor, &risks, &outcome.summary.changes, outcome.summary.error.as_deref(), &subject).await;
    if let Some(warning) = warning {
        em.notice(warning).await;
    }
    let verdict = explanation.verdict;
    em.send(StreamEvent::Inspection {
        result: Box::new(InspectionResult {
            perspective: perspective.to_string(),
            fee_payer: fee_payer.to_string(),
            perspective_signs: signs,
            instruction_count: message.instructions.len(),
            memos,
            simulation: outcome.summary,
            risks,
            explanation,
        }),
    })
    .await;
    em.stage("analyze", StageStatus::Done, Some(format!("{verdict:?}"))).await;
    Ok(())
}

pub async fn sample(st: &AppState, owner: &Pubkey) -> AppResult<SampleResponse> {
    portfolio::invalidate(st, &owner.to_string());
    let wallet = portfolio::load(st, owner).await?;
    let attacker = pseudo_random_key("sample-drainer");
    let mut instructions = vec![set_compute_unit_price(5_000)];
    let target = wallet
        .holdings
        .iter()
        .filter(|h| !h.native && h.account.is_some())
        .max_by(|a, b| a.usd_value.unwrap_or(0.0).total_cmp(&b.usd_value.unwrap_or(0.0)));
    if let Some(holding) = target {
        if let Some(account) = holding.account.as_deref().and_then(Pubkey::parse) {
            let program = Pubkey::parse(&holding.meta.token_program).unwrap_or(*TOKEN_PROGRAM);
            instructions.push(token_approve(account, attacker, *owner, u64::MAX, program));
        }
    }
    let lamports: u64 = wallet.holdings.first().and_then(|h| h.raw.parse().ok()).unwrap_or(0);
    let drain = lamports.saturating_sub(3_000_000) / 10 * 9;
    if drain > 0 {
        instructions.push(system_transfer(*owner, attacker, drain));
    }
    if instructions.len() == 1 {
        return user("This wallet is empty, so the sample drainer has nothing to take. Try another address.");
    }
    instructions.push(memo("Claim your 500 JUP airdrop reward now. Verify your wallet to receive it.", *owner));
    let (blockhash, _) = st.rpc.latest_blockhash().await?;
    let message = Message::compile(*owner, &instructions, &[], blockhash)?;
    Ok(SampleResponse {
        transaction: to_base64(&message.encode_unsigned()),
        description: "A fake airdrop claim that secretly grants unlimited spending of your tokens and sends most of your SOL to a stranger.".to_string(),
    })
}

pub async fn tamper(st: &AppState, request: &TamperRequest) -> AppResult<TamperResponse> {
    let owner = Pubkey::parse(&request.owner).ok_or_else(|| user_err("Invalid wallet address."))?;
    let message = decode(&request.transaction)?;
    let tables = load_tables(st, &message).await?;
    let mut instructions = message.decompile(&tables)?;
    let guarded = st
        .cfg
        .guard_program
        .is_some_and(|program| instructions.last().is_some_and(|ix| ix.program_id == program));
    let balance = st.rpc.balance(&owner).await?;
    let stolen = (balance / 4).clamp(1, 50_000_000);
    let attacker = pseudo_random_key("tamper");
    let position = if guarded { instructions.len() - 1 } else { instructions.len() };
    instructions.insert(position, system_transfer(owner, attacker, stolen));
    let (blockhash, _) = st.rpc.latest_blockhash().await?;
    let tampered = Message::compile(owner, &instructions, &tables, blockhash)?;
    let result = st.rpc.simulate(&to_base64(&tampered.encode_unsigned()), &[]).await?;
    let reverted = result.err.is_some();
    let sol = stolen as f64 / LAMPORTS_PER_SOL;
    let blocked_by_guard = result.logs.iter().any(|line| line.contains("Preflight guard check failed"));
    let (headline, detail) = match (guarded, reverted) {
        (true, true) if blocked_by_guard => (
            "Attack blocked by Preflight Guard",
            format!("A hidden transfer of {sol:.4} SOL was slipped in before the guard. The guard saw your balance fall below what you approved and reverted everything, so nothing would leave your wallet."),
        ),
        (_, true) => (
            "The tampered transaction failed",
            format!("The injected {sol:.4} SOL transfer made the transaction fail for another reason: {}", result.err.map(|e| simulate::friendly_error(&e, &result.logs)).unwrap_or_default()),
        ),
        (true, false) => (
            "The guard did not trigger",
            format!("The injected {sol:.4} SOL transfer stayed within the approved bounds."),
        ),
        (false, false) => (
            "Without the guard, the attack works",
            format!("This transaction has no Preflight Guard, so a hidden {sol:.4} SOL transfer would go through. Deploy the guard program to make previews binding."),
        ),
    };
    let start = result.logs.len().saturating_sub(10);
    Ok(TamperResponse {
        guarded,
        reverted,
        headline: headline.to_string(),
        detail,
        logs: result.logs[start..].to_vec(),
    })
}
