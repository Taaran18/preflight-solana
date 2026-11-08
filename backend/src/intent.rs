use crate::{
    portfolio::holding,
    solana::Pubkey,
    tokens::USDC_MINT,
    types::{AmountKind, AmountOf, Intent, IntentAction, Portfolio},
};

const STOP: &[&str] = &[
    "my", "of", "the", "some", "worth", "to", "for", "into", "with", "a", "an", "all", "half", "quarter", "is", "it",
    "safe", "check", "token", "coin", "about", "tell", "me", "sell", "buy", "swap", "convert", "trade", "exchange",
    "send", "pay", "transfer", "tip", "please", "legit", "rug", "rugpull", "scam", "info", "analyze", "analyse", "audit",
    "scan", "research", "what", "whats", "how", "and", "i", "want", "would", "like", "can", "you", "could", "everything",
    "max", "bucks", "slippage", "percent", "ape", "in", "on", "at", "this", "that", "now", "just", "get", "some", "few",
];

const USD_WORDS: &[&str] = &["usd", "dollars", "dollar", "bucks"];

fn has_any(words: &[String], list: &[&str]) -> bool {
    words.iter().any(|w| list.contains(&w.as_str()))
}

fn find_address(prompt: &str) -> Option<String> {
    prompt
        .split(|c: char| c.is_whitespace() || c == ',')
        .map(|w| w.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
        .find(|w| Pubkey::parse(w).is_some())
        .map(String::from)
}

fn format_amount(kind: Option<AmountKind>, value: Option<f64>) -> String {
    match (kind, value) {
        (Some(AmountKind::Usd), Some(v)) => format!("${v}"),
        (Some(AmountKind::Percent), Some(v)) => format!("{v}% of your"),
        (Some(AmountKind::All), _) => "all your".to_string(),
        (_, Some(v)) => v.to_string(),
        _ => String::new(),
    }
}

pub fn parse(prompt: &str, portfolio: Option<&Portfolio>) -> Intent {
    let lowered = prompt.to_lowercase();
    let raw_words: Vec<String> = prompt
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '?' | '!' | ';'))
        .map(|w| w.trim_matches(|c: char| matches!(c, '.' | '"' | '\'' | '(' | ')')).to_string())
        .filter(|w| !w.is_empty())
        .collect();
    let mut words: Vec<String> = raw_words.iter().map(|w| w.to_lowercase()).collect();
    let addresses: Vec<(usize, String)> = raw_words
        .iter()
        .enumerate()
        .filter(|(_, w)| Pubkey::parse(w).is_some())
        .map(|(i, w)| (i, w.clone()))
        .collect();
    let address = addresses.first().map(|(_, a)| a.clone()).or_else(|| find_address(prompt));

    let mut slippage_bps = None;
    if let Some(at) = words.iter().position(|w| w.starts_with("slippage")) {
        for neighbor in [at.checked_sub(1), Some(at + 1)].into_iter().flatten() {
            if let Some(value) = words.get(neighbor).and_then(|w| w.strip_suffix('%')).and_then(|v| v.parse::<f64>().ok()) {
                slippage_bps = Some((value * 100.0).round() as u32);
                words[neighbor].clear();
                break;
            }
        }
    }

    let mut amount_kind = None;
    let mut amount_value = None;
    let mut consumed: Option<usize> = None;
    for (i, word) in words.iter().enumerate() {
        if let Some(v) = word.strip_prefix('$').and_then(|rest| rest.parse::<f64>().ok()) {
            amount_kind = Some(AmountKind::Usd);
            amount_value = Some(v);
            break;
        }
        if let Some(v) = word.strip_suffix('%').and_then(|rest| rest.parse::<f64>().ok()) {
            amount_kind = Some(AmountKind::Percent);
            amount_value = Some(v);
            break;
        }
        if let Ok(v) = word.parse::<f64>() {
            let next_is_usd = words.get(i + 1).is_some_and(|n| USD_WORDS.contains(&n.as_str()));
            amount_kind = Some(if next_is_usd { AmountKind::Usd } else { AmountKind::Exact });
            amount_value = Some(v);
            if next_is_usd {
                consumed = Some(i + 1);
            }
            break;
        }
        match word.as_str() {
            "half" => {
                amount_kind = Some(AmountKind::Percent);
                amount_value = Some(50.0);
                break;
            }
            "quarter" => {
                amount_kind = Some(AmountKind::Percent);
                amount_value = Some(25.0);
                break;
            }
            "all" | "everything" | "max" => {
                amount_kind = Some(AmountKind::All);
                break;
            }
            _ => {}
        }
    }

    let mut candidates: Vec<(usize, String)> = words
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != consumed)
        .filter_map(|(i, w)| {
            let token = w.trim_start_matches('$');
            let valid = token.len() >= 2
                && token.len() <= 12
                && token.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && token.chars().all(|c| c.is_ascii_alphanumeric())
                && !STOP.contains(&token);
            valid.then(|| (i, token.to_uppercase()))
        })
        .collect();
    candidates.extend(addresses.iter().cloned());
    candidates.sort_by_key(|(i, _)| *i);

    let mut intent = Intent::reply(IntentAction::Unsupported, "");
    intent.slippage_bps = slippage_bps;

    if has_any(&words, &["portfolio", "balance", "balances", "holdings"]) || lowered.contains("what do i have") || lowered.contains("what do i own") {
        intent.action = IntentAction::Portfolio;
        intent.reply = match portfolio {
            Some(p) => {
                let top: Vec<String> = p.holdings.iter().take(3).map(|h| h.meta.symbol.clone()).collect();
                format!("You hold about ${:.2} across {} assets. Largest: {}.", p.total_usd, p.holdings.len(), top.join(", "))
            }
            None => "Connect a wallet and I'll show what you hold.".to_string(),
        };
        return intent;
    }

    if has_any(&words, &["send", "pay", "transfer", "tip"]) {
        let pivot = words.iter().position(|w| w == "to").unwrap_or(words.len());
        let recipient = addresses
            .iter()
            .find(|(i, _)| *i > pivot)
            .or_else(|| addresses.last())
            .map(|(_, a)| a.clone());
        let Some(recipient) = recipient else {
            intent.reply = "Who should receive it? Paste their full Solana address.".to_string();
            return intent;
        };
        if amount_kind.is_none() {
            intent.reply = "How much should I send?".to_string();
            return intent;
        }
        let token = candidates
            .iter()
            .filter(|(i, t)| *i < pivot && *t != recipient)
            .map(|(_, t)| t.clone())
            .last()
            .unwrap_or_else(|| "SOL".to_string());
        intent.action = IntentAction::Transfer;
        intent.reply = format!("Sending {} {token} to {}.", format_amount(amount_kind, amount_value), Pubkey::parse(&recipient).map(|k| k.short()).unwrap_or_default());
        intent.input_token = Some(token);
        intent.amount_kind = amount_kind;
        intent.amount_value = amount_value;
        intent.amount_of = Some(AmountOf::Input);
        intent.recipient = Some(recipient);
        return intent;
    }

    if has_any(&words, &["swap", "convert", "trade", "exchange", "sell", "buy", "ape"]) {
        let is_buy = has_any(&words, &["buy", "ape"]) && !has_any(&words, &["sell", "swap", "convert", "trade", "exchange"]);
        let (input, output) = if is_buy {
            let with_at = words.iter().position(|w| w == "with").unwrap_or(words.len());
            let output = address
                .clone()
                .or_else(|| candidates.iter().find(|(i, _)| *i < with_at).map(|(_, t)| t.clone()));
            let input = candidates.iter().find(|(i, _)| *i > with_at).map(|(_, t)| t.clone()).or_else(|| {
                let usdc = holding(portfolio, USDC_MINT).and_then(|h| h.usd_value).unwrap_or(0.0);
                let needed = if amount_kind == Some(AmountKind::Usd) { amount_value.unwrap_or(0.0) } else { 0.0 };
                Some(if usdc > 0.0 && usdc >= needed { "USDC" } else { "SOL" }.to_string())
            });
            (input, output)
        } else {
            let pivot = words.iter().position(|w| matches!(w.as_str(), "to" | "for" | "into"));
            let input = candidates
                .iter()
                .filter(|(i, _)| pivot.is_none_or(|p| *i < p))
                .map(|(_, t)| t.clone())
                .last()
                .or_else(|| address.clone());
            let output = pivot
                .and_then(|p| candidates.iter().find(|(i, _)| *i > p).map(|(_, t)| t.clone()))
                .or_else(|| has_any(&words, &["sell"]).then(|| "USDC".to_string()));
            (input, output)
        };
        let Some(output) = output else {
            intent.reply = "Which token should I swap into?".to_string();
            return intent;
        };
        if amount_kind.is_none() {
            intent.reply = "How much would you like to swap?".to_string();
            return intent;
        }
        let input = input.unwrap_or_else(|| "SOL".to_string());
        intent.action = IntentAction::Swap;
        intent.amount_of = Some(if is_buy && amount_kind == Some(AmountKind::Exact) { AmountOf::Output } else { AmountOf::Input });
        intent.reply = if is_buy {
            format!("Buying {} {output} with {input}.", format_amount(amount_kind, amount_value))
        } else {
            format!("Swapping {} {input} to {output}.", format_amount(amount_kind, amount_value))
        };
        intent.input_token = Some(input);
        intent.output_token = Some(output);
        intent.amount_kind = amount_kind;
        intent.amount_value = amount_value;
        return intent;
    }

    let wants_report = has_any(&words, &["safe", "legit", "rug", "rugpull", "scam", "check", "analyze", "analyse", "audit", "scan", "research", "about"]);
    if address.is_some() || wants_report {
        if let Some(token) = address.clone().or_else(|| candidates.last().map(|(_, t)| t.clone())) {
            intent.action = IntentAction::TokenReport;
            intent.reply = format!("Running a safety check on {}.", Pubkey::parse(&token).map(|k| k.short()).unwrap_or_else(|| token.clone()));
            intent.output_token = Some(token);
            return intent;
        }
    }

    intent.reply = "I can swap tokens (\"swap 0.1 SOL to USDC\"), send them (\"send 5 USDC to <address>\"), check a token (\"is BONK safe?\") or show your portfolio.".to_string();
    intent
}
