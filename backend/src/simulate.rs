use std::collections::HashMap;

use serde_json::Value;

use crate::{
    AppState,
    error::AppResult,
    solana::{LAMPORTS_PER_SOL, LookupTable, Message, Pubkey, SOL_MINT, is_token_program, program_name, to_base64},
    tokens::meta,
    types::{AuthorityEvent, AuthorityKind, BalanceChange, ProgramRef, SimulationSummary},
};

struct TokenState {
    mint: String,
    owner: Pubkey,
    amount: u64,
    delegate: Option<Pubkey>,
    delegated: u64,
    close_authority: Option<Pubkey>,
}

fn parse_token(program: &Pubkey, data: &[u8]) -> Option<TokenState> {
    if !is_token_program(program) || data.len() < 165 || (data.len() > 165 && data[165] != 2) {
        return None;
    }
    let delegate = if data[72..76] != [0, 0, 0, 0] {
        Pubkey::from_slice(&data[76..108])
    } else {
        None
    };
    Some(TokenState {
        mint: Pubkey::from_slice(&data[0..32])?.to_string(),
        owner: Pubkey::from_slice(&data[32..64])?,
        amount: u64::from_le_bytes(data[64..72].try_into().ok()?),
        delegate,
        delegated: u64::from_le_bytes(data[121..129].try_into().ok()?),
        close_authority: if data[129..133] != [0, 0, 0, 0] {
            Pubkey::from_slice(&data[133..165])
        } else {
            None
        },
    })
}

pub struct WatchedToken {
    pub account: Pubkey,
    pub mint: String,
    pub pre: Option<u64>,
    pub post: Option<u64>,
}

pub struct SimOutcome {
    pub summary: SimulationSummary,
    pub units: Option<u64>,
    pub pre_lamports: u64,
    pub post_lamports: Option<u64>,
    pub tokens: Vec<WatchedToken>,
    pub decimals: HashMap<String, u8>,
    pub symbols: HashMap<String, String>,
}

struct RawEvent {
    kind: AuthorityKind,
    mint: String,
    account: Pubkey,
    to: Option<Pubkey>,
    amount: u64,
}

pub async fn run(st: &AppState, message: &Message, tables: &[LookupTable], owner: &Pubkey) -> AppResult<SimOutcome> {
    let keys = message.account_keys(tables)?;
    let mut watch = vec![*owner];
    for (index, key) in keys.iter().enumerate() {
        if message.is_writable(index) && !watch.contains(key) {
            watch.push(*key);
        }
    }
    watch.truncate(100);

    let encoded = to_base64(&message.encode_unsigned());
    let (before, simulated) = tokio::join!(st.rpc.accounts(&watch), st.rpc.simulate(&encoded, &watch));
    let before = before?;
    let simulated = simulated?;
    let ok = simulated.err.is_none();
    let after = if ok { simulated.accounts.clone().unwrap_or_default() } else { Vec::new() };

    let pre_lamports = before.first().and_then(Option::as_ref).map(|a| a.lamports).unwrap_or(0);
    let post_lamports = ok.then(|| after.first().and_then(Option::as_ref).map(|a| a.lamports).unwrap_or(0));

    let mut tokens = Vec::new();
    let mut raw_events = Vec::new();
    if ok {
        let owner_before = before.first().and_then(Option::as_ref).map(|a| a.owner);
        let owner_after = after.first().and_then(Option::as_ref).map(|a| a.owner);
        if let (Some(old), Some(new)) = (owner_before, owner_after) {
            if old != new {
                raw_events.push(RawEvent {
                    kind: AuthorityKind::WalletAssigned,
                    mint: SOL_MINT.to_string(),
                    account: *owner,
                    to: Some(new),
                    amount: post_lamports.unwrap_or(0),
                });
            }
        }
    }
    for (index, address) in watch.iter().enumerate() {
        let pre = before.get(index).and_then(Option::as_ref).and_then(|a| parse_token(&a.owner, &a.data));
        let post = after
            .get(index)
            .and_then(Option::as_ref)
            .filter(|a| a.lamports > 0)
            .and_then(|a| parse_token(&a.owner, &a.data));
        let user_pre = pre.as_ref().is_some_and(|t| t.owner == *owner);
        let user_post = post.as_ref().is_some_and(|t| t.owner == *owner);
        if !user_pre && !user_post {
            continue;
        }
        let mint = pre.as_ref().or(post.as_ref()).map(|t| t.mint.clone()).unwrap_or_default();
        if ok {
            if let (true, Some(state)) = (user_pre, post.as_ref()) {
                if state.owner != *owner {
                    raw_events.push(RawEvent {
                        kind: AuthorityKind::OwnerChange,
                        mint: mint.clone(),
                        account: *address,
                        to: Some(state.owner),
                        amount: pre.as_ref().map(|t| t.amount).unwrap_or(0),
                    });
                }
            }
            if let Some(state) = post.as_ref().filter(|_| user_post) {
                if let Some(delegate) = state.delegate {
                    let changed = pre
                        .as_ref()
                        .map(|b| b.delegate != state.delegate || b.delegated != state.delegated)
                        .unwrap_or(true);
                    if changed && state.delegated > 0 {
                        raw_events.push(RawEvent {
                            kind: AuthorityKind::Delegate,
                            mint: mint.clone(),
                            account: *address,
                            to: Some(delegate),
                            amount: state.delegated,
                        });
                    }
                }
            }
            if let Some(state) = post.as_ref().filter(|_| user_post) {
                if let Some(authority) = state.close_authority {
                    let changed = pre.as_ref().map(|b| b.close_authority != state.close_authority).unwrap_or(true);
                    if changed && authority != *owner {
                        raw_events.push(RawEvent {
                            kind: AuthorityKind::CloseAuthority,
                            mint: mint.clone(),
                            account: *address,
                            to: Some(authority),
                            amount: state.amount,
                        });
                    }
                }
            }
            if user_pre && post.is_none() && mint != SOL_MINT {
                raw_events.push(RawEvent {
                    kind: AuthorityKind::Close,
                    mint: mint.clone(),
                    account: *address,
                    to: None,
                    amount: pre.as_ref().map(|t| t.amount).unwrap_or(0),
                });
            }
        }
        tokens.push(WatchedToken {
            account: *address,
            mint,
            pre: pre.filter(|_| user_pre).map(|t| t.amount),
            post: post.filter(|_| user_post).map(|t| t.amount),
        });
    }

    let mut mints = vec![SOL_MINT.to_string()];
    for token in &tokens {
        if !mints.contains(&token.mint) {
            mints.push(token.mint.clone());
        }
    }
    let metas = st.jup.tokens(&mints).await.unwrap_or_default();
    let mut decimals: HashMap<String, u8> = metas.iter().map(|(mint, token)| (mint.clone(), token.decimals)).collect();
    decimals.insert(SOL_MINT.to_string(), 9);
    let missing: Vec<Pubkey> = mints
        .iter()
        .filter(|mint| !decimals.contains_key(*mint))
        .filter_map(|mint| Pubkey::parse(mint))
        .collect();
    if !missing.is_empty() {
        if let Ok(accounts) = st.rpc.accounts(&missing).await {
            for (key, account) in missing.iter().zip(accounts) {
                if let Some(account) = account.filter(|a| a.data.len() > 44) {
                    decimals.insert(key.to_string(), account.data[44]);
                }
            }
        }
    }
    let mut symbols: HashMap<String, String> = mints
        .iter()
        .map(|mint| {
            let symbol = metas
                .get(mint)
                .map(|token| meta(token).symbol)
                .unwrap_or_else(|| mint.chars().take(4).collect());
            (mint.clone(), symbol)
        })
        .collect();
    symbols.insert(SOL_MINT.to_string(), "wSOL".to_string());

    let to_ui = |mint: &str, raw: u128| raw as f64 / 10f64.powi(decimals.get(mint).copied().unwrap_or(0) as i32);
    let mut changes = Vec::new();
    if let Some(post) = post_lamports.filter(|post| *post != pre_lamports) {
        let before_ui = pre_lamports as f64 / LAMPORTS_PER_SOL;
        let after_ui = post as f64 / LAMPORTS_PER_SOL;
        let delta = after_ui - before_ui;
        let sol = metas.get(SOL_MINT);
        changes.push(BalanceChange {
            mint: SOL_MINT.to_string(),
            symbol: "SOL".to_string(),
            icon: sol.and_then(|t| t.icon.clone()),
            native: true,
            verified: true,
            before: before_ui,
            after: after_ui,
            delta,
            usd_delta: sol.and_then(|t| t.usd_price).map(|price| price * delta),
        });
    }
    if ok {
        let mut per_mint: Vec<(String, u128, u128)> = Vec::new();
        for token in &tokens {
            let pre = token.pre.unwrap_or(0) as u128;
            let post = token.post.unwrap_or(0) as u128;
            match per_mint.iter_mut().find(|(mint, _, _)| *mint == token.mint) {
                Some(entry) => {
                    entry.1 += pre;
                    entry.2 += post;
                }
                None => per_mint.push((token.mint.clone(), pre, post)),
            }
        }
        for (mint, pre, post) in per_mint.into_iter().filter(|(_, pre, post)| pre != post) {
            let token = metas.get(&mint).map(meta);
            let before = to_ui(&mint, pre);
            let after = to_ui(&mint, post);
            let delta = after - before;
            changes.push(BalanceChange {
                symbol: symbols.get(&mint).cloned().unwrap_or_default(),
                icon: token.as_ref().and_then(|t| t.icon.clone()),
                native: false,
                verified: token.as_ref().map(|t| t.verified).unwrap_or(false),
                before,
                after,
                delta,
                usd_delta: token.as_ref().and_then(|t| t.usd_price).map(|price| price * delta),
                mint,
            });
        }
    }

    let events = raw_events
        .into_iter()
        .map(|event| AuthorityEvent {
            kind: event.kind,
            symbol: symbols.get(&event.mint).cloned().unwrap_or_default(),
            account: event.account.to_string(),
            to: event.to.map(|key| key.to_string()),
            amount: Some(to_ui(&event.mint, event.amount as u128)),
            unlimited: event.amount >= u64::MAX / 2,
            mint: event.mint,
        })
        .collect();

    let mut programs: Vec<ProgramRef> = Vec::new();
    let mut note_program = |id: String, top_level: bool| {
        if let Some(existing) = programs.iter_mut().find(|p| p.id == id) {
            existing.top_level |= top_level;
        } else {
            programs.push(ProgramRef {
                name: program_name(&id).map(String::from),
                id,
                top_level,
            });
        }
    };
    for ix in &message.instructions {
        if let Some(key) = keys.get(ix.program_index as usize) {
            note_program(key.to_string(), true);
        }
    }
    for line in &simulated.logs {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 && parts[0] == "Program" && parts[2] == "invoke" {
            note_program(parts[1].to_string(), parts[3] == "[1]");
        }
    }

    let error = simulated.err.as_ref().map(|err| friendly_error(err, &simulated.logs));
    let start = simulated.logs.len().saturating_sub(14);
    let summary = SimulationSummary {
        ok,
        error,
        units_consumed: simulated.units,
        changes,
        programs,
        events,
        logs_tail: simulated.logs[start..].to_vec(),
    };

    Ok(SimOutcome {
        summary,
        units: simulated.units,
        pre_lamports,
        post_lamports,
        tokens,
        decimals,
        symbols,
    })
}

pub fn friendly_error(err: &Value, logs: &[String]) -> String {
    let raw = err.to_string();
    let has = |needle: &str| logs.iter().any(|line| line.contains(needle));
    if has("Preflight guard check failed") {
        return "Preflight Guard blocked it: the on-chain result would differ from the approved preview.".to_string();
    }
    if raw.contains("AccountNotFound") {
        return "This wallet has no SOL on mainnet, so it can't pay network fees.".to_string();
    }
    if has("insufficient lamports") || raw.contains("InsufficientFundsForRent") || raw.contains("InsufficientFundsForFee") {
        return "Not enough SOL to cover this transaction and its network fees.".to_string();
    }
    if has("SlippageToleranceExceeded") || has("0x1771") {
        return "The price moved beyond the slippage tolerance before the swap could fill.".to_string();
    }
    if logs.iter().any(|line| line.ends_with("custom program error: 0x1")) || has("Error: insufficient funds") {
        return "Insufficient token balance for this transaction.".to_string();
    }
    format!("Simulation failed: {raw}")
}
