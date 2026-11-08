use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{
    AppState,
    error::AppResult,
    solana::{LAMPORTS_PER_SOL, Pubkey, SOL_MINT, SYSTEM_PROGRAM, TOKEN_2022_PROGRAM, TOKEN_PROGRAM},
    time::now_ms,
    tokens::meta,
    types::{Holding, Portfolio, TokenMeta},
};

struct Position {
    raw: u128,
    decimals: u8,
    program: Pubkey,
    account: Pubkey,
    largest: u64,
}

fn cached(st: &AppState, owner: &str) -> Option<Portfolio> {
    let cache = st.portfolios.lock().ok()?;
    cache
        .get(owner)
        .filter(|(at, _)| at.elapsed() < Duration::from_secs(8))
        .map(|(_, portfolio)| portfolio.clone())
}

pub fn invalidate(st: &AppState, owner: &str) {
    if let Ok(mut cache) = st.portfolios.lock() {
        cache.remove(owner);
    }
}

pub async fn load(st: &AppState, owner: &Pubkey) -> AppResult<Portfolio> {
    let owner_id = owner.to_string();
    if let Some(portfolio) = cached(st, &owner_id) {
        return Ok(portfolio);
    }

    let (lamports, classic, extended) = tokio::join!(
        st.rpc.balance(owner),
        st.rpc.token_accounts(owner, &TOKEN_PROGRAM),
        st.rpc.token_accounts(owner, &TOKEN_2022_PROGRAM),
    );
    let lamports = lamports?;
    let mut accounts = classic?;
    accounts.extend(extended.unwrap_or_default());

    let mut positions: HashMap<String, Position> = HashMap::new();
    for account in accounts.into_iter().filter(|a| a.amount > 0) {
        let entry = positions.entry(account.mint.clone()).or_insert(Position {
            raw: 0,
            decimals: account.decimals,
            program: account.program,
            account: account.address,
            largest: 0,
        });
        entry.raw += account.amount as u128;
        if account.amount > entry.largest {
            entry.largest = account.amount;
            entry.account = account.address;
        }
    }

    let mut mints = vec![SOL_MINT.to_string()];
    mints.extend(positions.keys().take(99).cloned());
    let metas = st.jup.tokens(&mints).await.unwrap_or_default();

    let sol = lamports as f64 / LAMPORTS_PER_SOL;
    let sol_meta = metas.get(SOL_MINT);
    let sol_price = sol_meta.and_then(|t| t.usd_price);
    let mut holdings = vec![Holding {
        meta: TokenMeta {
            mint: SOL_MINT.to_string(),
            symbol: "SOL".to_string(),
            name: "Solana".to_string(),
            decimals: 9,
            icon: sol_meta.and_then(|t| t.icon.clone()),
            verified: true,
            token_program: SYSTEM_PROGRAM.to_string(),
            usd_price: sol_price,
        },
        amount: sol,
        raw: lamports.to_string(),
        usd_value: sol_price.map(|p| p * sol),
        native: true,
        account: None,
        account_raw: None,
    }];

    let mut tokens: Vec<Holding> = positions
        .into_iter()
        .map(|(mint, position)| {
            let mut token_meta = metas.get(&mint).map(meta).unwrap_or_else(|| TokenMeta {
                symbol: mint.chars().take(4).collect(),
                name: "Unknown token".to_string(),
                mint: mint.clone(),
                decimals: position.decimals,
                icon: None,
                verified: false,
                token_program: position.program.to_string(),
                usd_price: None,
            });
            if mint == SOL_MINT {
                token_meta.symbol = "wSOL".to_string();
                token_meta.name = "Wrapped SOL".to_string();
            }
            token_meta.decimals = position.decimals;
            let amount = position.raw as f64 / 10f64.powi(position.decimals as i32);
            let usd_value = token_meta.usd_price.map(|p| p * amount);
            Holding {
                meta: token_meta,
                amount,
                raw: position.raw.to_string(),
                usd_value,
                native: false,
                account: Some(position.account.to_string()),
                account_raw: Some(position.largest.to_string()),
            }
        })
        .collect();
    tokens.sort_by(|a, b| {
        b.meta
            .verified
            .cmp(&a.meta.verified)
            .then(b.usd_value.unwrap_or(-1.0).total_cmp(&a.usd_value.unwrap_or(-1.0)))
    });
    holdings.extend(tokens);

    let total_usd = holdings.iter().filter_map(|h| h.usd_value).sum();
    let portfolio = Portfolio {
        owner: owner_id.clone(),
        sol,
        holdings,
        total_usd,
        fetched_at: now_ms(),
    };
    if let Ok(mut cache) = st.portfolios.lock() {
        cache.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(60));
        cache.insert(owner_id, (Instant::now(), portfolio.clone()));
    }
    Ok(portfolio)
}

pub fn holding<'a>(portfolio: Option<&'a Portfolio>, mint: &str) -> Option<&'a Holding> {
    portfolio?
        .holdings
        .iter()
        .find(|h| if mint == SOL_MINT { h.native } else { h.meta.mint == mint && !h.native })
}
