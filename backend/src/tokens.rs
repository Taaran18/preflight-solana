use crate::{
    AppState,
    error::{AppResult, user},
    jupiter::JupToken,
    solana::{Pubkey, SOL_MINT, TOKEN_2022_PROGRAM, TOKEN_PROGRAM, is_token_program},
    time::age_days,
    types::{TokenMeta, TokenStats},
};

pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
const PYUSD_MINT: &str = "2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo";
const MSOL_MINT: &str = "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So";
const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";
const JUPSOL_MINT: &str = "jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v";

const KNOWN: &[(&str, &str)] = &[
    ("SOL", SOL_MINT),
    ("WSOL", SOL_MINT),
    ("SOLANA", SOL_MINT),
    ("USDC", USDC_MINT),
    ("USD", USDC_MINT),
    ("DOLLARS", USDC_MINT),
    ("USDT", USDT_MINT),
    ("PYUSD", PYUSD_MINT),
    ("MSOL", MSOL_MINT),
    ("JITOSOL", JITOSOL_MINT),
    ("JUPSOL", JUPSOL_MINT),
    ("JUP", "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"),
    ("BONK", "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),
    ("WIF", "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm"),
    ("JTO", "jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL"),
    ("PYTH", "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3"),
    ("RAY", "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"),
    ("RENDER", "rndrizKT3MK1iimdxRdWabcF7Zg7AR5T4nud4EkHBof"),
];

const TRUSTED: &[&str] = &[SOL_MINT, USDC_MINT, USDT_MINT, PYUSD_MINT, MSOL_MINT, JITOSOL_MINT, JUPSOL_MINT];

pub fn known_mint(symbol: &str) -> Option<&'static str> {
    let wanted = symbol.trim().trim_start_matches('$').to_uppercase();
    KNOWN.iter().find(|(name, _)| *name == wanted).map(|(_, mint)| *mint)
}

pub fn is_trusted(mint: &str) -> bool {
    TRUSTED.contains(&mint)
}

pub fn meta(token: &JupToken) -> TokenMeta {
    let symbol = if token.symbol.is_empty() {
        token.id.chars().take(4).collect()
    } else {
        token.symbol.clone()
    };
    TokenMeta {
        mint: token.id.clone(),
        symbol,
        name: if token.name.is_empty() { "Unknown token".to_string() } else { token.name.clone() },
        decimals: token.decimals,
        icon: token.icon.clone().filter(|icon| icon.starts_with("https://")),
        verified: token.is_verified.unwrap_or(false) || is_trusted(&token.id),
        token_program: if token.token_program.is_empty() {
            TOKEN_PROGRAM.to_string()
        } else {
            token.token_program.clone()
        },
        usd_price: token.usd_price,
    }
}

pub struct Resolved {
    pub info: JupToken,
    pub lookalikes: usize,
}

pub async fn resolve(st: &AppState, query: &str) -> AppResult<Resolved> {
    let query = query.trim().trim_start_matches('$').to_string();
    if query.is_empty() {
        return user("Which token do you mean?");
    }
    if let Some(mint) = Pubkey::parse(&query) {
        let id = mint.to_string();
        let found = st.jup.tokens(std::slice::from_ref(&id)).await.unwrap_or_default();
        if let Some(token) = found.get(&id) {
            return Ok(Resolved {
                info: token.clone(),
                lookalikes: 0,
            });
        }
        return Ok(Resolved {
            info: onchain_token(st, &mint).await?,
            lookalikes: 0,
        });
    }
    if let Some(mint) = known_mint(&query) {
        let found = st.jup.tokens(&[mint.to_string()]).await?;
        if let Some(token) = found.get(mint) {
            return Ok(Resolved {
                info: token.clone(),
                lookalikes: 0,
            });
        }
    }
    let wanted = query.to_uppercase();
    let results = st.jup.search(&query).await?;
    let mut pool: Vec<JupToken> = results.iter().filter(|t| t.symbol.to_uppercase() == wanted).cloned().collect();
    if pool.is_empty() {
        pool = results.iter().filter(|t| t.name.to_uppercase() == wanted).cloned().collect();
    }
    if pool.is_empty() {
        return user(format!("I couldn't find a token called \"{query}\". Try pasting its mint address."));
    }
    pool.sort_by(|a, b| {
        b.is_verified
            .unwrap_or(false)
            .cmp(&a.is_verified.unwrap_or(false))
            .then(b.liquidity.unwrap_or(0.0).total_cmp(&a.liquidity.unwrap_or(0.0)))
    });
    let lookalikes = pool.len() - 1;
    Ok(Resolved {
        info: pool.swap_remove(0),
        lookalikes,
    })
}

async fn onchain_token(st: &AppState, mint: &Pubkey) -> AppResult<JupToken> {
    let Some(account) = st.rpc.account(mint).await? else {
        return user(format!("No account exists at {}. Is that a token mint?", mint.short()));
    };
    if !is_token_program(&account.owner) {
        return user(format!("{} is not a token mint.", mint.short()));
    }
    let Some(facts) = parse_mint(&account.data, account.owner == *TOKEN_2022_PROGRAM) else {
        return user(format!("{} is a token account, not a token mint.", mint.short()));
    };
    Ok(JupToken {
        id: mint.to_string(),
        name: "Unlisted token".to_string(),
        symbol: mint.to_string().chars().take(4).collect(),
        decimals: facts.decimals,
        token_program: account.owner.to_string(),
        is_verified: Some(false),
        mint_authority: facts.mint_authority.map(|k| k.to_string()),
        freeze_authority: facts.freeze_authority.map(|k| k.to_string()),
        ..Default::default()
    })
}

#[derive(Clone, Debug)]
pub struct MintFacts {
    pub token_2022: bool,
    pub mint_authority: Option<Pubkey>,
    pub freeze_authority: Option<Pubkey>,
    pub decimals: u8,
    pub extensions: Vec<String>,
    pub transfer_fee_bps: Option<u16>,
    pub permanent_delegate: Option<Pubkey>,
    pub transfer_hook: Option<Pubkey>,
    pub default_frozen: bool,
    pub non_transferable: bool,
}

fn option_key(data: &[u8], tag_at: usize) -> Option<Pubkey> {
    let tag = data.get(tag_at..tag_at + 4)?;
    if tag == [0, 0, 0, 0] {
        return None;
    }
    Pubkey::from_slice(data.get(tag_at + 4..tag_at + 36)?)
}

fn extension_name(kind: u16) -> String {
    match kind {
        1 => "TransferFee",
        2 => "TransferFeeAmount",
        3 => "MintCloseAuthority",
        4 => "ConfidentialTransfer",
        6 => "DefaultAccountState",
        9 => "NonTransferable",
        10 => "InterestBearing",
        12 => "PermanentDelegate",
        14 => "TransferHook",
        16 => "ConfidentialTransferFee",
        18 => "MetadataPointer",
        19 => "TokenMetadata",
        20 => "GroupPointer",
        21 => "TokenGroup",
        22 => "GroupMemberPointer",
        23 => "TokenGroupMember",
        25 => "ScaledUiAmount",
        26 => "Pausable",
        _ => return format!("Extension{kind}"),
    }
    .to_string()
}

pub fn parse_mint(data: &[u8], token_2022: bool) -> Option<MintFacts> {
    let plain_mint = data.len() == 82;
    let extended_mint = data.len() > 165 && data[165] == 1;
    if !plain_mint && !extended_mint {
        return None;
    }
    let mut facts = MintFacts {
        token_2022,
        mint_authority: option_key(data, 0),
        freeze_authority: option_key(data, 46),
        decimals: data[44],
        extensions: Vec::new(),
        transfer_fee_bps: None,
        permanent_delegate: None,
        transfer_hook: None,
        default_frozen: false,
        non_transferable: false,
    };
    if token_2022 && data.len() > 166 {
        let mut pos = 166;
        while pos + 4 <= data.len() {
            let kind = u16::from_le_bytes([data[pos], data[pos + 1]]);
            let len = u16::from_le_bytes([data[pos + 2], data[pos + 3]]) as usize;
            let start = pos + 4;
            let end = start + len;
            if kind == 0 || end > data.len() {
                break;
            }
            let value = &data[start..end];
            facts.extensions.push(extension_name(kind));
            match kind {
                1 if value.len() >= 108 => {
                    facts.transfer_fee_bps = Some(u16::from_le_bytes([value[106], value[107]]));
                }
                6 if !value.is_empty() => facts.default_frozen = value[0] == 2,
                9 => facts.non_transferable = true,
                12 => {
                    facts.permanent_delegate = Pubkey::from_slice(value.get(0..32).unwrap_or_default())
                        .filter(|key| key.0 != [0u8; 32]);
                }
                14 => {
                    facts.transfer_hook = Pubkey::from_slice(value.get(32..64).unwrap_or_default())
                        .filter(|key| key.0 != [0u8; 32]);
                }
                _ => {}
            }
            pos = end;
        }
    }
    Some(facts)
}

pub async fn mint_facts(st: &AppState, mint: &str) -> Option<MintFacts> {
    let key = Pubkey::parse(mint)?;
    let account = st.rpc.account(&key).await.ok()??;
    if !is_token_program(&account.owner) {
        return None;
    }
    parse_mint(&account.data, account.owner == *TOKEN_2022_PROGRAM)
}

pub fn stats(token: &JupToken, facts: Option<&MintFacts>) -> TokenStats {
    let created = token
        .first_pool
        .as_ref()
        .and_then(|pool| pool.created_at.as_deref())
        .or(token.created_at.as_deref());
    TokenStats {
        liquidity_usd: token.liquidity,
        mcap_usd: token.mcap,
        holders: token.holder_count,
        age_days: age_days(created),
        organic_score: token.organic_score,
        organic_label: token.organic_score_label.clone(),
        top_holders_pct: token.audit.as_ref().and_then(|a| a.top_holders_percentage),
        price_change_24h: token.stats_24h.as_ref().and_then(|s| s.price_change),
        program: Some(if facts.map(|f| f.token_2022).unwrap_or(token.token_program == TOKEN_2022_PROGRAM.to_string()) {
            "Token-2022".to_string()
        } else {
            "SPL Token".to_string()
        }),
        extensions: facts.map(|f| f.extensions.clone()).unwrap_or_default(),
        mint_authority: facts
            .map(|f| f.mint_authority.map(|k| k.to_string()))
            .unwrap_or_else(|| token.mint_authority.clone()),
        freeze_authority: facts
            .map(|f| f.freeze_authority.map(|k| k.to_string()))
            .unwrap_or_else(|| token.freeze_authority.clone()),
    }
}
