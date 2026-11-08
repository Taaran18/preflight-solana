use crate::{
    jupiter::JupToken,
    solana::Pubkey,
    time::age_days,
    tokens::{MintFacts, is_trusted},
    types::{AuthorityKind, QuoteInfo, RiskFlag, RiskLevel, SimulationSummary, Verdict},
};

use RiskLevel::{Danger, Info, Ok, Warn};

fn flag(level: RiskLevel, code: &str, title: impl Into<String>, detail: impl Into<String>) -> RiskFlag {
    RiskFlag::new(level, code, title, detail)
}

fn short(value: &str) -> String {
    Pubkey::parse(value).map(|key| key.short()).unwrap_or_else(|| value.to_string())
}

pub fn compact_usd(value: f64) -> String {
    match value {
        v if v >= 1e9 => format!("${:.1}B", v / 1e9),
        v if v >= 1e6 => format!("${:.1}M", v / 1e6),
        v if v >= 1e3 => format!("${:.1}K", v / 1e3),
        v => format!("${v:.0}"),
    }
}

pub fn token_risks(token: &JupToken, facts: Option<&MintFacts>) -> Vec<RiskFlag> {
    let mut out = Vec::new();
    let symbol = token.symbol.as_str();
    let trusted = is_trusted(&token.id);
    let verified = token.is_verified.unwrap_or(false) || trusted;
    let audit = token.audit.clone().unwrap_or_default();

    if audit.is_sus.unwrap_or(false) {
        out.push(flag(Danger, "token_suspicious", format!("{symbol} is flagged as suspicious"), "Jupiter's token audit marks this token as suspicious."));
    }
    if verified {
        out.push(flag(Ok, "token_verified", format!("{symbol} is a verified token"), "Listed and verified in Jupiter's token registry."));
    } else {
        out.push(flag(Warn, "token_unverified", format!("{symbol} is not verified"), "Unverified tokens are often impersonations or low-quality launches. Check the mint address carefully."));
    }

    let mint_active = facts
        .map(|f| f.mint_authority.is_some())
        .or_else(|| audit.mint_authority_disabled.map(|disabled| !disabled));
    match mint_active {
        Some(true) if trusted => out.push(flag(Info, "mint_authority_issuer", "Issuer can mint new supply", format!("Normal for {symbol}: regulated stablecoins and staking tokens are minted on demand."))),
        Some(true) => out.push(flag(Warn, "mint_authority", "Supply can be inflated", "The creator still holds mint authority and can create unlimited new tokens, diluting every holder.")),
        Some(false) => out.push(flag(Ok, "mint_fixed", "Fixed supply", "Mint authority is disabled, so no one can create more tokens.")),
        None => {}
    }

    let freeze_active = facts
        .map(|f| f.freeze_authority.is_some())
        .or_else(|| audit.freeze_authority_disabled.map(|disabled| !disabled));
    match freeze_active {
        Some(true) if trusted => out.push(flag(Info, "freeze_authority_issuer", "Issuer can freeze accounts", "Expected for regulated stablecoins, which must be able to freeze funds for compliance.")),
        Some(true) => out.push(flag(Danger, "freeze_authority", "Creator can freeze your tokens", "Freeze authority is active: the creator can lock your balance so you can never sell. This is a classic honeypot pattern.")),
        Some(false) => out.push(flag(Ok, "freeze_none", "No freeze authority", "Nobody can lock the tokens in your wallet.")),
        None => {}
    }

    if let Some(facts) = facts {
        if let Some(delegate) = facts.permanent_delegate {
            out.push(flag(
                if trusted { Warn } else { Danger },
                "permanent_delegate",
                "Issuer can move your tokens",
                format!("Token-2022 permanent delegate {} can transfer or burn tokens from any holder at any time.", delegate.short()),
            ));
        }
        if let Some(bps) = facts.transfer_fee_bps.filter(|bps| *bps > 0) {
            out.push(flag(Warn, "transfer_fee", format!("{}% fee on every transfer", bps as f64 / 100.0), "Part of every transfer is withheld for the token's fee authority."));
        }
        if let Some(hook) = facts.transfer_hook {
            out.push(flag(Warn, "transfer_hook", "Custom code runs on every transfer", format!("Transfers call program {}, which can block or alter them.", hook.short())));
        }
        if facts.non_transferable {
            out.push(flag(Danger, "non_transferable", "Token can't be transferred", "This is a soulbound token. You would never be able to sell it."));
        }
        if facts.default_frozen {
            out.push(flag(Danger, "default_frozen", "New holders start frozen", "Accounts start frozen until the issuer approves them."));
        }
        if facts.extensions.iter().any(|e| e == "Pausable") {
            out.push(flag(
                if trusted { Warn } else { Danger },
                "pausable",
                "Issuer can pause all transfers",
                "A pause switch can stop everyone from moving or selling this token at once.",
            ));
        }
        if facts.extensions.iter().any(|e| e == "ScaledUiAmount" || e == "InterestBearing") {
            out.push(flag(Warn, "scaled_amount", "Displayed balance is scaled", "This token shows a balance adjusted by an interest or scaling rate, so amounts may differ from raw units."));
        }
    }

    if let Some(liquidity) = token.liquidity {
        if liquidity < 10_000.0 {
            out.push(flag(Danger, "liquidity_low", format!("Very thin liquidity ({})", compact_usd(liquidity)), "Even a small sale could crash the price or fail outright."));
        } else if liquidity < 100_000.0 {
            out.push(flag(Warn, "liquidity_thin", format!("Thin liquidity ({})", compact_usd(liquidity)), "Larger trades will move the price noticeably."));
        } else {
            out.push(flag(Ok, "liquidity_ok", format!("Deep liquidity ({})", compact_usd(liquidity)), "Plenty of on-chain liquidity for normal trade sizes."));
        }
    }
    if !trusted {
        if let Some(top) = audit.top_holders_percentage.filter(|top| *top > 50.0) {
            out.push(flag(Warn, "holders_concentrated", format!("Top holders own {top:.0}% of supply"), "A handful of wallets could dump and crash the price."));
        }
        if let Some(dev) = audit.dev_balance_percentage.filter(|dev| *dev > 10.0) {
            out.push(flag(Warn, "dev_balance", format!("Developer holds {dev:.0}% of supply"), "The creator can sell a large position into the market at any time."));
        }
    }
    if !verified && token.organic_score_label.as_deref() == Some("low") {
        out.push(flag(Warn, "organic_low", "Mostly inorganic trading", "Jupiter's organic score suggests much of the volume comes from bots or wash trading."));
    }
    let created = token
        .first_pool
        .as_ref()
        .and_then(|pool| pool.created_at.as_deref())
        .or(token.created_at.as_deref());
    if let Some(age) = age_days(created) {
        if age < 1.0 {
            out.push(flag(Warn, "token_new", format!("Brand-new market ({:.0}h old)", (age * 24.0).max(1.0)), "Most rug pulls happen within the first days of trading."));
        } else if age < 7.0 && !verified {
            out.push(flag(Info, "token_young", format!("Young market ({age:.0} days old)"), "There is little trading history to judge this token by."));
        }
    }
    if let Some(change) = token.stats_24h.as_ref().and_then(|s| s.price_change).filter(|c| *c < -40.0) {
        out.push(flag(Warn, "price_crash", format!("Down {:.0}% in 24h", change.abs()), "Sharp drops often follow a dump by insiders."));
    }
    out
}

pub fn lookalike_risk(token: &JupToken, lookalikes: usize) -> Option<RiskFlag> {
    if lookalikes == 0 {
        return None;
    }
    let picked = if token.is_verified.unwrap_or(false) { "verified" } else { "most liquid" };
    Some(flag(
        Warn,
        "ticker_lookalikes",
        format!("{lookalikes} other tokens use the ticker {}", token.symbol),
        format!("Picked the {picked} one ({}). Scammers copy popular tickers, so confirm the mint.", short(&token.id)),
    ))
}

pub fn swap_risks(quote: &QuoteInfo, portfolio_usd: Option<f64>, sol_after: Option<f64>) -> Vec<RiskFlag> {
    let mut out = Vec::new();
    let impact = quote.price_impact_pct;
    if impact >= 5.0 {
        out.push(flag(Danger, "price_impact_high", format!("{impact:.2}% price impact"), "This trade moves the market a lot, so you'd get far less than the market price. Try a smaller amount."));
    } else if impact >= 1.0 {
        out.push(flag(Warn, "price_impact", format!("{impact:.2}% price impact"), "Your trade is large relative to available liquidity."));
    } else {
        out.push(flag(Ok, "price_impact_low", format!("Low price impact ({impact:.2}%)"), "Your trade barely moves the market price."));
    }
    let slippage = quote.slippage_bps as f64 / 100.0;
    if quote.slippage_bps > 1000 {
        out.push(flag(Danger, "slippage_high", format!("{slippage}% slippage tolerance"), "Bots can exploit wide slippage to fill you at a much worse price."));
    } else if quote.slippage_bps > 300 {
        out.push(flag(Warn, "slippage_wide", format!("{slippage}% slippage tolerance"), "Wider than usual. You may receive noticeably less than quoted."));
    } else {
        out.push(flag(Ok, "slippage_ok", format!("Protected by {slippage}% slippage limit"), "If the price moves further before the swap lands, it fails instead of filling worse."));
    }
    if let (Some(spend), Some(total)) = (quote.in_usd, portfolio_usd) {
        if total > 0.0 && spend / total > 0.5 {
            out.push(flag(Info, "large_share", format!("Uses {:.0}% of your portfolio", spend / total * 100.0), "A big position change. Make sure that's intended."));
        }
    }
    if let Some(sol) = sol_after.filter(|sol| *sol < 0.005) {
        out.push(flag(Warn, "sol_low_after", "Almost no SOL left for fees", format!("You'd keep about {sol:.4} SOL, which may not cover future network fees.")));
    }
    out
}

pub struct RecipientFacts {
    pub on_curve: bool,
    pub exists: bool,
    pub executable: bool,
    pub token_account_or_mint: bool,
    pub recent_txs: Option<usize>,
    pub is_self: bool,
}

pub fn recipient_risks(r: &RecipientFacts) -> Vec<RiskFlag> {
    let mut out = Vec::new();
    if r.is_self {
        out.push(flag(Warn, "self_transfer", "You're sending to yourself", "The recipient is the same wallet that pays. Nothing would change except fees."));
    }
    if r.executable {
        out.push(flag(Danger, "recipient_program", "Recipient is a program, not a wallet", "Funds sent to a program address are usually lost forever."));
    } else if r.token_account_or_mint {
        out.push(flag(Danger, "recipient_token_account", "Recipient is a token account, not a wallet", "Wallets need the owner's address. Tokens sent here may be unrecoverable."));
    } else if !r.on_curve {
        out.push(flag(Warn, "recipient_pda", "Recipient is a program-controlled address", "This isn't a normal wallet. Only continue if the app you're paying gave you this address."));
    }
    match r.recent_txs {
        Some(0) if !r.exists => out.push(flag(Warn, "recipient_new", "Brand-new address with no history", "Check every character. Address-poisoning scams use look-alike addresses.")),
        Some(count) if count > 0 => out.push(flag(Ok, "recipient_active", "Recipient has on-chain history", "This address has been used before.")),
        _ => {}
    }
    out
}

pub fn simulation_risks(sim: &SimulationSummary, inspection: bool, pre_sol: f64) -> Vec<RiskFlag> {
    let mut out = Vec::new();
    if sim.ok {
        out.push(flag(Ok, "simulation_ok", "Simulated against live mainnet", "The transaction succeeds against the current on-chain state."));
    } else {
        out.push(flag(Danger, "simulation_failed", "Simulation failed", sim.error.clone().unwrap_or_default()));
    }
    for event in &sim.events {
        let to = event.to.as_deref().map(short).unwrap_or_else(|| "someone".to_string());
        match event.kind {
            AuthorityKind::OwnerChange => out.push(flag(
                Danger,
                "owner_change",
                format!("Hands your {} account to {to}", event.symbol),
                "Whoever owns a token account controls every token in it. This is how wallet drainers work.",
            )),
            AuthorityKind::Delegate if event.unlimited => out.push(flag(
                Danger,
                "unlimited_approval",
                format!("Unlimited spending approval for your {}", event.symbol),
                format!("{to} could move all of your {} at any time, without asking again.", event.symbol),
            )),
            AuthorityKind::Delegate => out.push(flag(
                Warn,
                "approval",
                format!("Lets {to} spend your {}", event.symbol),
                format!("Approves up to {:.4} {} to be moved by another address.", event.amount.unwrap_or(0.0), event.symbol),
            )),
            AuthorityKind::Close => out.push(flag(
                Info,
                "account_closed",
                format!("Closes your {} token account", event.symbol),
                "The small rent deposit goes to the address named in the instruction.",
            )),
            AuthorityKind::WalletAssigned => out.push(flag(
                Danger,
                "wallet_assigned",
                format!("Hands control of your wallet to program {to}"),
                "Your wallet account would be reassigned to another program, which could then move your SOL without asking you.",
            )),
            AuthorityKind::CloseAuthority => out.push(flag(
                Danger,
                "close_authority",
                format!("Lets {to} close your {} account", event.symbol),
                "The close authority can shut the account and take its balance, especially for wrapped SOL.",
            )),
        }
    }
    if inspection {
        for program in sim.programs.iter().filter(|p| p.top_level && p.name.is_none()) {
            out.push(flag(Warn, "unknown_program", format!("Calls an unrecognized program ({})", short(&program.id)), "Unknown programs can do anything they're coded to do with the accounts you hand them."));
        }
        if let Some(sol) = sim.changes.iter().find(|c| c.native && c.delta < 0.0) {
            let spent = -sol.delta;
            let share = if pre_sol > 0.0 { spent / pre_sol } else { 0.0 };
            if share >= 0.5 {
                out.push(flag(Danger, "sol_drain", format!("Moves {:.0}% of your SOL out", share * 100.0), format!("{spent:.4} SOL would leave your wallet.")));
            } else if spent > 0.015 {
                out.push(flag(Warn, "sol_outflow", format!("Sends {spent:.4} SOL out of your wallet"), "More than fees and rent. Make sure you expect to pay this."));
            } else if spent > 0.0 {
                out.push(flag(Info, "sol_fees", format!("Spends {spent:.6} SOL"), "Network fees and any rent for new accounts."));
            }
        }
        for change in sim.changes.iter().filter(|c| !c.native && c.delta < 0.0) {
            let spent = -change.delta;
            let share = if change.before > 0.0 { spent / change.before } else { 1.0 };
            if share >= 0.5 {
                out.push(flag(
                    Danger,
                    "token_drain",
                    format!("Moves {:.0}% of your {} out", share * 100.0, change.symbol),
                    format!("{spent:.4} {} would leave your wallet.", change.symbol),
                ));
            } else {
                out.push(flag(
                    Warn,
                    "token_outflow",
                    format!("Sends {spent:.4} {} out of your wallet", change.symbol),
                    "Make sure you expect this token to leave your wallet.",
                ));
            }
        }
        for change in sim.changes.iter().filter(|c| !c.native && c.delta > 0.0 && !c.verified) {
            out.push(flag(Warn, "unverified_incoming", format!("You'd receive an unverified token ({})", change.symbol), "Unknown airdropped tokens are a common bait for phishing sites."));
        }
    }
    out
}

pub fn sort(risks: &mut [RiskFlag]) {
    risks.sort_by(|a, b| b.level.cmp(&a.level));
}

pub fn verdict(risks: &[RiskFlag], sim_ok: bool) -> Verdict {
    if !sim_ok || risks.iter().any(|r| r.level == Danger) {
        Verdict::Danger
    } else if risks.iter().any(|r| r.level == Warn) {
        Verdict::Caution
    } else {
        Verdict::Safe
    }
}
