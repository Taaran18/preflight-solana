use serde_json::{Value, json};

use crate::{
    AppState, Shared, ai,
    error::{AppResult, user, user_err},
    guard::{self, GuardContext},
    intent,
    jupiter::{self, JupToken, number},
    portfolio,
    risk::{self, RecipientFacts},
    simulate::{self, SimOutcome},
    solana::{
        Instruction, LAMPORTS_PER_SOL, LookupTable, MAX_COMPUTE_UNITS, Message, PACKET_LIMIT, Pubkey, SOL_MINT,
        TOKEN_PROGRAM, associated_token_address, compute_unit_price, create_ata_idempotent, hex, is_token_program, memo,
        set_compute_unit_price, sha256, system_transfer, to_base64, token_transfer_checked, with_compute_limit,
    },
    stream::Emitter,
    time::now_ms,
    tokens::{self, MintFacts},
    types::{
        AmountKind, AmountOf, Explanation, Intent, IntentAction, PlanKind, PlanRequest, PlanResult, Portfolio, QuoteInfo,
        Receipt, ReceiptMode, RiskFlag, SimulationSummary, StageStatus, StreamEvent, TokenReport, Verdict,
    },
};

const SOL_RESERVE: f64 = 0.01;
const QUOTE_TTL_MS: u64 = 55_000;
const SWAP_MAX_ACCOUNTS: u32 = 40;
const TRANSFER_PRIORITY: u64 = 100_000;

pub async fn run(st: Shared, request: PlanRequest, em: Emitter) -> AppResult<()> {
    em.stages(&[("understand", "Understand")]).await;
    em.stage("understand", StageStatus::Active, None).await;

    let prompt = request.prompt.trim().to_string();
    if prompt.is_empty() {
        return user("Tell me what you'd like to do.");
    }
    if prompt.len() > 2000 {
        return user("That message is too long. Keep it to a sentence or two.");
    }
    let owner = match request.owner.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(value) => Some(Pubkey::parse(value).ok_or_else(|| user_err("That wallet address isn't valid."))?),
        None => None,
    };
    let portfolio = match owner {
        Some(owner) => match portfolio::load(&st, &owner).await {
            Ok(portfolio) => Some(portfolio),
            Err(err) => {
                em.notice(format!("Couldn't load wallet balances: {}", err.message())).await;
                None
            }
        },
        None => None,
    };

    if em.closed() {
        return Ok(());
    }
    let (intent, parsed_by) = if ai::enabled(&st) {
        match ai::parse_intent(&st, &prompt, owner.as_ref(), portfolio.as_ref()).await {
            Ok(intent) => (intent, st.cfg.model.clone()),
            Err(err) => {
                em.notice(format!("Claude was unavailable ({err}). Using the built-in parser.")).await;
                (intent::parse(&prompt, portfolio.as_ref()), "rules".to_string())
            }
        }
    } else {
        (intent::parse(&prompt, portfolio.as_ref()), "rules".to_string())
    };
    em.send(StreamEvent::Intent {
        intent: intent.clone(),
        parsed_by,
    })
    .await;
    em.stage("understand", StageStatus::Done, Some(intent.reply.clone())).await;

    match intent.action {
        IntentAction::Swap => swap(&st, &prompt, &intent, owner, portfolio.as_ref(), &em).await,
        IntentAction::Transfer => transfer(&st, &prompt, &intent, owner, portfolio.as_ref(), &em).await,
        IntentAction::TokenReport => token_report(&st, &intent, &em).await,
        IntentAction::Portfolio => {
            em.send(StreamEvent::Message { text: intent.reply.clone() }).await;
            if let Some(portfolio) = portfolio {
                em.send(StreamEvent::Portfolio { portfolio }).await;
            }
            Ok(())
        }
        IntentAction::Unsupported => {
            em.send(StreamEvent::Message { text: intent.reply.clone() }).await;
            Ok(())
        }
    }
}

struct Amount {
    raw: u64,
    ui: f64,
    note: Option<String>,
}

fn trim_number(value: f64) -> String {
    let text = if value.abs() >= 1000.0 {
        format!("{value:.2}")
    } else if value.abs() >= 1.0 {
        format!("{value:.4}")
    } else {
        format!("{value:.6}")
    };
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn compute_amount(intent: &Intent, input: &JupToken, output: Option<&JupToken>, portfolio: Option<&Portfolio>) -> AppResult<Amount> {
    let is_sol = input.id == SOL_MINT;
    let scale = 10f64.powi(input.decimals as i32);
    let held = portfolio::holding(portfolio, &input.id);
    let balance_raw: u64 = held
        .and_then(|h| h.account_raw.as_deref().or(Some(h.raw.as_str())))
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(0);
    let balance = if is_sol { held.map(|h| h.amount).unwrap_or(0.0) } else { balance_raw as f64 / scale };
    let spendable = if is_sol { (balance - SOL_RESERVE).max(0.0) } else { balance };
    let spendable_raw = if is_sol { (spendable * scale).floor() as u64 } else { balance_raw };
    let kind = intent
        .amount_kind
        .ok_or_else(|| user_err("How much? Try something like \"0.1 SOL\", \"half\" or \"$20\"."))?;
    let value = intent.amount_value;
    let mut note = None;

    let ui = match kind {
        AmountKind::Exact => {
            let v = value.ok_or_else(|| user_err("How much?"))?;
            if intent.amount_of == Some(AmountOf::Output) {
                let output = output.ok_or_else(|| user_err("Which token do you want to receive?"))?;
                match (input.usd_price, output.usd_price) {
                    (Some(pay), Some(get)) if pay > 0.0 => {
                        note = Some(format!("About {} {} at current prices", trim_number(v), output.symbol));
                        v * get / pay
                    }
                    _ => return user(format!("I can't price {} right now. Give the amount in {} instead.", output.symbol, input.symbol)),
                }
            } else {
                v
            }
        }
        AmountKind::Percent => {
            let v = value.ok_or_else(|| user_err("What percentage?"))?;
            if !(0.0..=100.0).contains(&v) {
                return user("Percentages must be between 0 and 100.");
            }
            note = Some(format!("{}% of your spendable {}", trim_number(v), input.symbol));
            if v >= 100.0 && !is_sol && portfolio.is_some() && balance_raw > 0 {
                return Ok(Amount {
                    raw: balance_raw,
                    ui: balance,
                    note,
                });
            }
            spendable * v / 100.0
        }
        AmountKind::Usd => {
            let v = value.ok_or_else(|| user_err("How many dollars?"))?;
            let price = input
                .usd_price
                .filter(|p| *p > 0.0)
                .ok_or_else(|| user_err(format!("No USD price is available for {}.", input.symbol)))?;
            note = Some(format!("${} worth at ${} per {}", trim_number(v), trim_number(price), input.symbol));
            v / price
        }
        AmountKind::All => {
            if !is_sol && portfolio.is_some() && balance_raw > 0 {
                return Ok(Amount {
                    raw: balance_raw,
                    ui: balance,
                    note: Some("Your entire balance".to_string()),
                });
            }
            if is_sol {
                note = Some(format!("Keeping {SOL_RESERVE} SOL for network fees"));
            }
            spendable
        }
    };

    if !(ui > 0.0) {
        if portfolio.is_some() && balance <= 0.0 {
            return user(format!("You don't hold any {}.", input.symbol));
        }
        return user("The amount has to be greater than zero.");
    }
    if portfolio.is_some() && ui > spendable * 1.000_001 + 1e-12 {
        let reserve = if is_sol { format!(" ({} spendable after fees)", trim_number(spendable)) } else { String::new() };
        return user(format!(
            "Not enough {}: you have {}{reserve}, but this needs {}.",
            input.symbol,
            trim_number(balance),
            trim_number(ui)
        ));
    }
    let mut raw = (ui * scale).round() as u64;
    if portfolio.is_some() && raw > spendable_raw {
        raw = spendable_raw;
    }
    if raw == 0 {
        return user("That amount is too small to send.");
    }
    Ok(Amount { raw, ui, note })
}

fn verdict_label(verdict: Verdict) -> String {
    match verdict {
        Verdict::Safe => "Looks safe",
        Verdict::Caution => "Caution",
        Verdict::Danger => "High risk",
    }
    .to_string()
}

fn simulation_note(outcome: &SimOutcome) -> String {
    if outcome.summary.ok {
        let units = outcome.summary.units_consumed.unwrap_or(0);
        format!("{} balance changes · {} CU", outcome.summary.changes.len(), units)
    } else {
        outcome.summary.error.clone().unwrap_or_else(|| "Simulation failed".to_string())
    }
}

pub fn simulation_json(summary: &SimulationSummary) -> Value {
    json!({
        "ok": summary.ok,
        "error": summary.error,
        "balanceChanges": summary.changes.iter().map(|c| json!({
            "token": c.symbol, "mint": c.mint, "verified": c.verified,
            "before": c.before, "after": c.after, "change": c.delta, "usdChange": c.usd_delta
        })).collect::<Vec<_>>(),
        "programs": summary.programs.iter().map(|p| p.name.clone().unwrap_or_else(|| format!("unknown program {}", p.id))).collect::<Vec<_>>(),
        "authorityEvents": summary.events,
    })
}

pub fn risks_json(risks: &[RiskFlag]) -> Value {
    json!(risks.iter().map(|r| json!({ "level": r.level, "title": r.title, "detail": r.detail })).collect::<Vec<_>>())
}

fn token_json(token: &JupToken, facts: Option<&MintFacts>) -> Value {
    let stats = tokens::stats(token, facts);
    json!({
        "symbol": token.symbol,
        "name": token.name,
        "mint": token.id,
        "verified": token.is_verified.unwrap_or(false) || tokens::is_trusted(&token.id),
        "usdPrice": token.usd_price,
        "liquidityUsd": stats.liquidity_usd,
        "marketCapUsd": stats.mcap_usd,
        "holders": stats.holders,
        "marketAgeDays": stats.age_days,
        "organicScore": stats.organic_label,
        "topHoldersPercent": stats.top_holders_pct,
        "priceChange24hPercent": stats.price_change_24h,
        "program": stats.program,
        "extensions": stats.extensions,
        "mintAuthority": if stats.mint_authority.is_some() { "active" } else { "disabled" },
        "freezeAuthority": if stats.freeze_authority.is_some() { "active" } else { "disabled" },
    })
}

fn report_json(owner: &Pubkey, prompt: &str, kind: &str, summary: &SimulationSummary, risks: &[RiskFlag], explanation: &Explanation, quote: Value) -> String {
    json!({
        "app": "preflight",
        "version": 1,
        "network": "mainnet-beta",
        "wallet": owner.to_string(),
        "request": prompt,
        "kind": kind,
        "createdAt": now_ms(),
        "expectedChanges": summary.changes.iter().map(|c| json!({ "mint": c.mint, "symbol": c.symbol, "delta": c.delta })).collect::<Vec<_>>(),
        "quote": quote,
        "risks": risks.iter().map(|r| json!({ "level": r.level, "code": r.code })).collect::<Vec<_>>(),
        "verdict": explanation.verdict,
        "headline": explanation.headline,
        "summary": explanation.summary,
        "model": explanation.model,
    })
    .to_string()
}

struct Finalized {
    transaction: String,
    receipt: Receipt,
    fee_sol: f64,
    cu_limit: u32,
}

fn fee_sol(units: u32, micro_lamports: u64) -> f64 {
    (5_000.0 + units as f64 * micro_lamports as f64 / 1_000_000.0) / LAMPORTS_PER_SOL
}

enum Sealed {
    Ready { transaction: String, units: u32 },
    TooLarge,
    Failed(String),
}

async fn seal(
    st: &AppState,
    owner: Pubkey,
    instructions: &[Instruction],
    extra: Option<&Instruction>,
    tables: &[LookupTable],
    blockhash: [u8; 32],
) -> AppResult<Sealed> {
    let build = |units: u32| -> anyhow::Result<Vec<u8>> {
        let mut list = with_compute_limit(instructions, units);
        if let Some(ix) = extra {
            list.push(ix.clone());
        }
        Ok(Message::compile(owner, &list, tables, blockhash)?.encode_unsigned())
    };
    let probe = build(MAX_COMPUTE_UNITS)?;
    if probe.len() > PACKET_LIMIT {
        return Ok(Sealed::TooLarge);
    }
    let check = st.rpc.simulate(&to_base64(&probe), &[]).await?;
    if let Some(err) = check.err {
        return Ok(Sealed::Failed(simulate::friendly_error(&err, &check.logs)));
    }
    let used = check.units.unwrap_or(200_000) as f64;
    let units = ((used * 1.15) as u32 + 8_000).min(MAX_COMPUTE_UNITS);
    Ok(Sealed::Ready {
        transaction: to_base64(&build(units)?),
        units,
    })
}

#[allow(clippy::too_many_arguments)]
async fn finalize(
    st: &AppState,
    owner: Pubkey,
    instructions: &[Instruction],
    tables: &[LookupTable],
    blockhash: [u8; 32],
    outcome: &SimOutcome,
    report: String,
    ctx: GuardContext<'_>,
    em: &Emitter,
) -> AppResult<Finalized> {
    let digest = sha256(report.as_bytes());
    let hash = hex(&digest);
    let base_units = outcome
        .units
        .map(|units| (units as f64 * 1.2) as u32 + 20_000)
        .unwrap_or(400_000)
        .min(MAX_COMPUTE_UNITS);
    let price = compute_unit_price(instructions).unwrap_or(0);

    if outcome.summary.ok {
        if let Some(program) = st.cfg.guard_program {
            if let Some(plan) = guard::build(program, owner, &digest, outcome, &ctx) {
                match seal(st, owner, instructions, Some(&plan.instruction), tables, blockhash).await {
                    Ok(Sealed::Ready { transaction, units }) => {
                        return Ok(Finalized {
                            transaction,
                            receipt: Receipt {
                                hash,
                                mode: ReceiptMode::Guard,
                                program: Some(program.to_string()),
                                memo: None,
                                checks: plan.labels,
                                report,
                            },
                            fee_sol: fee_sol(units, price),
                            cu_limit: units,
                        });
                    }
                    Ok(Sealed::TooLarge) => {
                        em.notice("This route is too large to add Preflight Guard, so it carries a memo receipt instead.").await;
                    }
                    Ok(Sealed::Failed(reason)) => {
                        em.notice(format!("Preflight Guard didn't pass a re-check ({reason}). Using a memo receipt instead.")).await;
                    }
                    Err(err) => {
                        em.notice(format!("Couldn't re-check the guarded transaction ({}). Using a memo receipt instead.", err.message())).await;
                    }
                }
            }
        }

        let memo_text = format!("preflight:v1:{hash}");
        let memo_ix = memo(&memo_text, owner);
        match seal(st, owner, instructions, Some(&memo_ix), tables, blockhash).await? {
            Sealed::Ready { transaction, units } => {
                return Ok(Finalized {
                    transaction,
                    receipt: Receipt {
                        hash,
                        mode: ReceiptMode::Memo,
                        program: None,
                        memo: Some(memo_text),
                        checks: Vec::new(),
                        report,
                    },
                    fee_sol: fee_sol(units, price),
                    cu_limit: units,
                });
            }
            Sealed::TooLarge => {}
            Sealed::Failed(reason) => em.notice(format!("The memo receipt didn't fit ({reason}). Sending without it.")).await,
        }

        if let Sealed::Ready { transaction, units } = seal(st, owner, instructions, None, tables, blockhash).await? {
            return Ok(Finalized {
                transaction,
                receipt: Receipt {
                    hash,
                    mode: ReceiptMode::None,
                    program: None,
                    memo: None,
                    checks: Vec::new(),
                    report,
                },
                fee_sol: fee_sol(units, price),
                cu_limit: units,
            });
        }
    }

    let message = Message::compile(owner, &with_compute_limit(instructions, base_units), tables, blockhash)?;
    let bytes = message.encode_unsigned();
    if bytes.len() > PACKET_LIMIT {
        return user("This route is too large for a single transaction. Try a smaller amount or a more liquid token.");
    }
    Ok(Finalized {
        transaction: to_base64(&bytes),
        receipt: Receipt {
            hash,
            mode: ReceiptMode::None,
            program: None,
            memo: None,
            checks: Vec::new(),
            report,
        },
        fee_sol: fee_sol(base_units, price),
        cu_limit: base_units,
    })
}

async fn swap(st: &AppState, prompt: &str, intent: &Intent, owner: Option<Pubkey>, portfolio: Option<&Portfolio>, em: &Emitter) -> AppResult<()> {
    let owner = owner.ok_or_else(|| user_err("Connect a wallet, or enter an address to simulate as, to plan a swap."))?;
    em.stages(&[("understand", "Understand"), ("route", "Route"), ("simulate", "Simulate"), ("analyze", "Analyze")]).await;
    em.stage("route", StageStatus::Active, None).await;

    let output_query = intent.output_token.clone().ok_or_else(|| user_err("Which token do you want to receive?"))?;
    let input_query = intent.input_token.clone().unwrap_or_else(|| "SOL".to_string());
    let input = tokens::resolve(st, &input_query).await?;
    let output = tokens::resolve(st, &output_query).await?;
    if input.info.id == output.info.id {
        return user(format!("Both sides are {}. Pick a different token to swap into.", input.info.symbol));
    }
    let amount = compute_amount(intent, &input.info, Some(&output.info), portfolio)?;
    let build = st
        .jup
        .build(&input.info.id, &output.info.id, amount.raw, &owner, intent.slippage_bps, SWAP_MAX_ACCOUNTS)
        .await?;

    let to_ui = |raw: &str, decimals: u8| raw.parse::<f64>().unwrap_or(0.0) / 10f64.powi(decimals as i32);
    let in_amount = to_ui(&build.in_amount, input.info.decimals);
    let out_amount = to_ui(&build.out_amount, output.info.decimals);
    let min_out = to_ui(&build.other_amount_threshold, output.info.decimals);
    let slippage_bps = number(&build.slippage_bps).map(|v| v as u32).or(intent.slippage_bps).unwrap_or(50);
    let impact = number(&build.price_impact_pct).unwrap_or(0.0).abs() * 100.0;
    let mut route: Vec<String> = build.route_plan.iter().filter_map(|step| step.swap_info.label.clone()).collect();
    route.dedup();
    let quote = QuoteInfo {
        in_amount,
        out_amount,
        min_out,
        price_impact_pct: impact,
        slippage_bps,
        route: route.clone(),
        in_usd: input.info.usd_price.map(|p| p * in_amount),
        out_usd: output.info.usd_price.map(|p| p * out_amount),
    };
    let route_note = if route.is_empty() { "Best route found".to_string() } else { format!("via {}", route.join(" → ")) };
    em.stage("route", StageStatus::Done, Some(route_note)).await;

    em.stage("simulate", StageStatus::Active, None).await;
    let (instructions, tables) = jupiter::swap_instructions(&build)?;
    let (blockhash, last_valid) = st.rpc.latest_blockhash().await?;
    let draft = Message::compile(owner, &with_compute_limit(&instructions, MAX_COMPUTE_UNITS), &tables, blockhash)?;
    let outcome = simulate::run(st, &draft, &tables, &owner).await?;
    let sim_status = if outcome.summary.ok { StageStatus::Done } else { StageStatus::Error };
    em.stage("simulate", sim_status, Some(simulation_note(&outcome))).await;

    if em.closed() {
        return Ok(());
    }
    em.stage("analyze", StageStatus::Active, None).await;
    let facts = tokens::mint_facts(st, &output.info.id).await;
    let mut risks = risk::token_risks(&output.info, facts.as_ref());
    risks.extend(risk::lookalike_risk(&output.info, output.lookalikes));
    let sol_after = outcome.post_lamports.map(|l| l as f64 / LAMPORTS_PER_SOL);
    risks.extend(risk::swap_risks(&quote, portfolio.map(|p| p.total_usd), sol_after));
    risks.extend(risk::simulation_risks(&outcome.summary, false, outcome.pre_lamports as f64 / LAMPORTS_PER_SOL));
    risk::sort(&mut risks);
    let floor = risk::verdict(&risks, outcome.summary.ok);

    let input_meta = tokens::meta(&input.info);
    let output_meta = tokens::meta(&output.info);
    let subject = format!(
        "Swap {} {} for about {} {}",
        trim_number(in_amount),
        input_meta.symbol,
        trim_number(out_amount),
        output_meta.symbol
    );
    let facts_json = json!({
        "kind": "swap",
        "userRequest": prompt,
        "pay": { "token": input_meta.symbol, "amount": in_amount, "usd": quote.in_usd },
        "receiveEstimate": { "token": output_meta.symbol, "amount": out_amount, "usd": quote.out_usd, "minimumAfterSlippage": min_out },
        "amountNote": amount.note,
        "priceImpactPercent": impact,
        "slippagePercent": slippage_bps as f64 / 100.0,
        "route": route,
        "outputToken": token_json(&output.info, facts.as_ref()),
        "simulation": simulation_json(&outcome.summary),
        "riskFlags": risks_json(&risks),
        "ruleVerdict": floor,
        "onChainGuard": st.cfg.guard_program.is_some(),
    });
    let (explanation, warning) = ai::explain(st, &facts_json, floor, &risks, &outcome.summary.changes, outcome.summary.error.as_deref(), &subject).await;
    if let Some(warning) = warning {
        em.notice(warning).await;
    }

    let output_quoted = build.out_amount.parse::<u64>().unwrap_or(0);
    let output_threshold = build.other_amount_threshold.parse::<u64>().unwrap_or(0);
    let report = report_json(
        &owner,
        prompt,
        "swap",
        &outcome.summary,
        &risks,
        &explanation,
        json!({ "in": in_amount, "out": out_amount, "minOut": min_out, "route": quote.route }),
    );
    let finalized = finalize(
        st,
        owner,
        &instructions,
        &tables,
        blockhash,
        &outcome,
        report,
        GuardContext {
            output_mint: Some(output.info.id.as_str()),
            output_quoted_raw: output_quoted,
            output_threshold_raw: output_threshold,
            sol_slack_lamports: 3_000_000,
        },
        em,
    )
    .await?;

    let verdict = explanation.verdict;
    let plan = PlanResult {
        kind: PlanKind::Swap,
        owner: owner.to_string(),
        prompt: prompt.to_string(),
        input: input_meta,
        output: Some(output_meta),
        amount: in_amount,
        amount_note: amount.note,
        quote: Some(quote),
        recipient: None,
        simulation: outcome.summary,
        risks,
        explanation,
        receipt: finalized.receipt,
        transaction: finalized.transaction,
        last_valid_block_height: last_valid,
        expires_at: now_ms() + QUOTE_TTL_MS,
        fee_sol: finalized.fee_sol,
        cu_limit: finalized.cu_limit,
    };
    em.send(StreamEvent::Plan { plan: Box::new(plan) }).await;
    em.stage("analyze", StageStatus::Done, Some(verdict_label(verdict))).await;
    Ok(())
}

async fn recipient_facts(st: &AppState, recipient: &Pubkey, owner: &Pubkey) -> RecipientFacts {
    let (account, count) = tokio::join!(st.rpc.account(recipient), st.rpc.recent_signature_count(recipient, 5));
    let account = account.ok().flatten();
    RecipientFacts {
        on_curve: recipient.is_on_curve(),
        exists: account.is_some(),
        executable: account.as_ref().is_some_and(|a| a.executable),
        token_account_or_mint: account.as_ref().is_some_and(|a| is_token_program(&a.owner)),
        recent_txs: count.ok(),
        is_self: recipient == owner,
    }
}

async fn transfer(st: &AppState, prompt: &str, intent: &Intent, owner: Option<Pubkey>, portfolio: Option<&Portfolio>, em: &Emitter) -> AppResult<()> {
    let owner = owner.ok_or_else(|| user_err("Connect a wallet, or enter an address to simulate as, to plan a transfer."))?;
    em.stages(&[("understand", "Understand"), ("prepare", "Prepare"), ("simulate", "Simulate"), ("analyze", "Analyze")]).await;
    em.stage("prepare", StageStatus::Active, None).await;

    let recipient = intent
        .recipient
        .as_deref()
        .and_then(Pubkey::parse)
        .ok_or_else(|| user_err("Paste the full Solana address you want to send to."))?;
    let token = tokens::resolve(st, intent.input_token.as_deref().unwrap_or("SOL")).await?;
    let amount = compute_amount(intent, &token.info, None, portfolio)?;
    let meta = tokens::meta(&token.info);

    let mut instructions = vec![set_compute_unit_price(TRANSFER_PRIORITY)];
    if token.info.id == SOL_MINT {
        instructions.push(system_transfer(owner, recipient, amount.raw));
    } else {
        let mint = Pubkey::parse(&token.info.id).ok_or_else(|| user_err("Invalid token mint."))?;
        let program = Pubkey::parse(&meta.token_program).unwrap_or(*TOKEN_PROGRAM);
        let source = portfolio::holding(portfolio, &token.info.id)
            .and_then(|h| h.account.as_deref())
            .and_then(Pubkey::parse)
            .or_else(|| associated_token_address(&owner, &mint, &program))
            .ok_or_else(|| user_err(format!("Couldn't find your {} account.", meta.symbol)))?;
        let destination = associated_token_address(&recipient, &mint, &program)
            .ok_or_else(|| user_err("Couldn't derive the recipient's token account."))?;
        instructions.push(create_ata_idempotent(owner, destination, recipient, mint, program));
        instructions.push(token_transfer_checked(source, mint, destination, owner, amount.raw, meta.decimals, program));
    }
    let facts = recipient_facts(st, &recipient, &owner).await;
    em.stage(
        "prepare",
        StageStatus::Done,
        Some(format!("{} {} to {}", trim_number(amount.ui), meta.symbol, recipient.short())),
    )
    .await;

    em.stage("simulate", StageStatus::Active, None).await;
    let (blockhash, last_valid) = st.rpc.latest_blockhash().await?;
    let draft = Message::compile(owner, &with_compute_limit(&instructions, 200_000), &[], blockhash)?;
    let outcome = simulate::run(st, &draft, &[], &owner).await?;
    let sim_status = if outcome.summary.ok { StageStatus::Done } else { StageStatus::Error };
    em.stage("simulate", sim_status, Some(simulation_note(&outcome))).await;

    if em.closed() {
        return Ok(());
    }
    em.stage("analyze", StageStatus::Active, None).await;
    let mut risks = risk::recipient_risks(&facts);
    risks.extend(risk::simulation_risks(&outcome.summary, false, outcome.pre_lamports as f64 / LAMPORTS_PER_SOL));
    let usd = meta.usd_price.map(|p| p * amount.ui);
    if let (Some(usd), Some(total)) = (usd, portfolio.map(|p| p.total_usd)) {
        if total > 0.0 && usd / total > 0.5 {
            risks.push(RiskFlag::new(
                crate::types::RiskLevel::Info,
                "large_share",
                format!("Sends {:.0}% of your portfolio", usd / total * 100.0),
                "A big transfer. Make sure the address is right.",
            ));
        }
    }
    risk::sort(&mut risks);
    let floor = risk::verdict(&risks, outcome.summary.ok);
    let subject = format!("Send {} {} to {}", trim_number(amount.ui), meta.symbol, recipient.short());
    let facts_json = json!({
        "kind": "transfer",
        "userRequest": prompt,
        "send": { "token": meta.symbol, "amount": amount.ui, "usd": usd },
        "recipient": {
            "address": recipient.to_string(),
            "isNormalWallet": facts.on_curve && !facts.executable && !facts.token_account_or_mint,
            "isProgram": facts.executable,
            "isTokenAccountOrMint": facts.token_account_or_mint,
            "existsOnChain": facts.exists,
            "recentTransactions": facts.recent_txs,
            "isSender": facts.is_self,
        },
        "simulation": simulation_json(&outcome.summary),
        "riskFlags": risks_json(&risks),
        "ruleVerdict": floor,
        "onChainGuard": st.cfg.guard_program.is_some(),
    });
    let (explanation, warning) = ai::explain(st, &facts_json, floor, &risks, &outcome.summary.changes, outcome.summary.error.as_deref(), &subject).await;
    if let Some(warning) = warning {
        em.notice(warning).await;
    }

    let report = report_json(
        &owner,
        prompt,
        "transfer",
        &outcome.summary,
        &risks,
        &explanation,
        json!({ "amount": amount.ui, "token": meta.mint, "recipient": recipient.to_string() }),
    );
    let finalized = finalize(
        st,
        owner,
        &instructions,
        &[],
        blockhash,
        &outcome,
        report,
        GuardContext {
            output_mint: None,
            output_quoted_raw: 0,
            output_threshold_raw: 0,
            sol_slack_lamports: 1_000_000,
        },
        em,
    )
    .await?;

    let verdict = explanation.verdict;
    let plan = PlanResult {
        kind: PlanKind::Transfer,
        owner: owner.to_string(),
        prompt: prompt.to_string(),
        input: meta,
        output: None,
        amount: amount.ui,
        amount_note: amount.note,
        quote: None,
        recipient: Some(recipient.to_string()),
        simulation: outcome.summary,
        risks,
        explanation,
        receipt: finalized.receipt,
        transaction: finalized.transaction,
        last_valid_block_height: last_valid,
        expires_at: now_ms() + QUOTE_TTL_MS,
        fee_sol: finalized.fee_sol,
        cu_limit: finalized.cu_limit,
    };
    em.send(StreamEvent::Plan { plan: Box::new(plan) }).await;
    em.stage("analyze", StageStatus::Done, Some(verdict_label(verdict))).await;
    Ok(())
}

async fn token_report(st: &AppState, intent: &Intent, em: &Emitter) -> AppResult<()> {
    em.stages(&[("understand", "Understand"), ("lookup", "Look Up"), ("audit", "Audit"), ("analyze", "Analyze")]).await;
    em.stage("lookup", StageStatus::Active, None).await;
    let query = intent
        .output_token
        .clone()
        .or_else(|| intent.input_token.clone())
        .ok_or_else(|| user_err("Which token should I check?"))?;
    let resolved = tokens::resolve(st, &query).await?;
    let meta = tokens::meta(&resolved.info);
    em.stage("lookup", StageStatus::Done, Some(format!("{} · {}", meta.symbol, meta.name))).await;

    em.stage("audit", StageStatus::Active, None).await;
    let facts = tokens::mint_facts(st, &resolved.info.id).await;
    let mut risks = risk::token_risks(&resolved.info, facts.as_ref());
    risks.extend(risk::lookalike_risk(&resolved.info, resolved.lookalikes));
    risk::sort(&mut risks);
    let stats = tokens::stats(&resolved.info, facts.as_ref());
    em.stage("audit", StageStatus::Done, Some(format!("{} checks run", risks.len()))).await;

    if em.closed() {
        return Ok(());
    }
    em.stage("analyze", StageStatus::Active, None).await;
    let floor = risk::verdict(&risks, true);
    let facts_json = json!({
        "kind": "token_report",
        "token": token_json(&resolved.info, facts.as_ref()),
        "otherTokensWithSameTicker": resolved.lookalikes,
        "riskFlags": risks_json(&risks),
        "ruleVerdict": floor,
    });
    let subject = format!("Safety report for {} ({})", meta.symbol, meta.name);
    let (explanation, warning) = ai::explain(st, &facts_json, floor, &risks, &[], None, &subject).await;
    if let Some(warning) = warning {
        em.notice(warning).await;
    }
    let verdict = explanation.verdict;
    em.send(StreamEvent::TokenReport {
        report: Box::new(TokenReport {
            token: meta,
            stats,
            risks,
            explanation,
            lookalikes: resolved.lookalikes,
        }),
    })
    .await;
    em.stage("analyze", StageStatus::Done, Some(verdict_label(verdict))).await;
    Ok(())
}
