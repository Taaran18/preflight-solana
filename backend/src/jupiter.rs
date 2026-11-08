use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    config::Config,
    error::{AppError, AppResult},
    solana::{AccountMeta, Instruction, LookupTable, Pubkey, from_base64},
};

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct JupAudit {
    pub is_sus: Option<bool>,
    pub mint_authority_disabled: Option<bool>,
    pub freeze_authority_disabled: Option<bool>,
    pub top_holders_percentage: Option<f64>,
    pub dev_balance_percentage: Option<f64>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct JupStats {
    pub price_change: Option<f64>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct JupPool {
    pub created_at: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct JupToken {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub symbol: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub decimals: u8,
    #[serde(default)]
    pub token_program: String,
    pub is_verified: Option<bool>,
    pub organic_score: Option<f64>,
    pub organic_score_label: Option<String>,
    pub usd_price: Option<f64>,
    pub liquidity: Option<f64>,
    pub mcap: Option<f64>,
    pub holder_count: Option<f64>,
    pub created_at: Option<String>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub audit: Option<JupAudit>,
    #[serde(rename = "stats24h")]
    pub stats_24h: Option<JupStats>,
    pub first_pool: Option<JupPool>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JupAccount {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JupInstruction {
    pub program_id: String,
    pub accounts: Vec<JupAccount>,
    pub data: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    pub label: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RouteStep {
    #[serde(default)]
    pub swap_info: SwapInfo,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JupBuild {
    pub in_amount: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    #[serde(default)]
    pub slippage_bps: Value,
    #[serde(default)]
    pub price_impact_pct: Value,
    #[serde(default)]
    pub route_plan: Vec<RouteStep>,
    #[serde(default)]
    pub compute_budget_instructions: Vec<JupInstruction>,
    #[serde(default)]
    pub setup_instructions: Vec<JupInstruction>,
    pub swap_instruction: JupInstruction,
    pub cleanup_instruction: Option<JupInstruction>,
    #[serde(default)]
    pub other_instructions: Vec<JupInstruction>,
    #[serde(default)]
    pub addresses_by_lookup_table_address: Option<HashMap<String, Vec<String>>>,
}

pub fn number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

pub struct Jupiter {
    http: reqwest::Client,
    base: String,
    key: Option<String>,
    interval: Duration,
    next_slot: Mutex<Instant>,
    cache: Mutex<HashMap<String, (Instant, JupToken)>>,
}

impl Jupiter {
    pub fn new(http: reqwest::Client, cfg: &Config) -> Self {
        Self {
            http,
            base: cfg.jup_base.trim_end_matches('/').to_string(),
            key: cfg.jup_key.clone(),
            interval: Duration::from_secs_f64(1.0 / cfg.jup_rps.max(0.1)),
            next_slot: Mutex::new(Instant::now()),
            cache: Mutex::new(HashMap::new()),
        }
    }

    async fn wait_turn(&self) {
        let wait = {
            let mut slot = self.next_slot.lock().await;
            let now = Instant::now();
            let start = if *slot > now { *slot } else { now };
            *slot = start + self.interval;
            start.saturating_duration_since(now)
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> AppResult<Value> {
        let url = format!("{}{}", self.base, path);
        for attempt in 0..4u64 {
            self.wait_turn().await;
            let mut request = self.http.get(&url).query(query);
            if let Some(key) = &self.key {
                request = request.header("x-api-key", key);
            }
            let response = request.send().await?;
            let status = response.status();
            if status.as_u16() == 429 {
                tokio::time::sleep(Duration::from_millis(700 * (attempt + 1))).await;
                continue;
            }
            let text = response.text().await?;
            if !status.is_success() {
                let detail = serde_json::from_str::<Value>(&text)
                    .ok()
                    .and_then(|v| {
                        v.get("error")
                            .or_else(|| v.get("message"))
                            .and_then(Value::as_str)
                            .map(String::from)
                    })
                    .unwrap_or_else(|| text.chars().take(160).collect());
                return Err(AppError::User(format!("Jupiter: {detail}")));
            }
            return Ok(serde_json::from_str(&text)?);
        }
        Err(AppError::User(
            "Jupiter is rate-limiting requests. Add a free JUPITER_API_KEY to backend/.env.".to_string(),
        ))
    }

    pub async fn search(&self, query: &str) -> AppResult<Vec<JupToken>> {
        let value = self.get("/tokens/v2/search", &[("query", query.to_string())]).await?;
        let tokens: Vec<JupToken> = value
            .as_array()
            .map(|items| items.iter().filter_map(|item| serde_json::from_value(item.clone()).ok()).collect())
            .unwrap_or_default();
        let mut cache = self.cache.lock().await;
        let now = Instant::now();
        for token in &tokens {
            cache.insert(token.id.clone(), (now, token.clone()));
        }
        Ok(tokens)
    }

    pub async fn tokens(&self, mints: &[String]) -> AppResult<HashMap<String, JupToken>> {
        let mut found = HashMap::new();
        let mut missing: Vec<String> = Vec::new();
        {
            let cache = self.cache.lock().await;
            for mint in mints {
                match cache.get(mint) {
                    Some((at, token)) if at.elapsed() < Duration::from_secs(60) => {
                        found.insert(mint.clone(), token.clone());
                    }
                    _ => {
                        if !missing.contains(mint) {
                            missing.push(mint.clone());
                        }
                    }
                }
            }
        }
        for chunk in missing.chunks(100) {
            for token in self.search(&chunk.join(",")).await? {
                found.insert(token.id.clone(), token);
            }
        }
        Ok(found)
    }

    pub async fn build(
        &self,
        input: &str,
        output: &str,
        amount: u64,
        taker: &Pubkey,
        slippage_bps: Option<u32>,
        max_accounts: u32,
    ) -> AppResult<JupBuild> {
        let mut query = vec![
            ("inputMint", input.to_string()),
            ("outputMint", output.to_string()),
            ("amount", amount.to_string()),
            ("taker", taker.to_string()),
            ("maxAccounts", max_accounts.to_string()),
        ];
        if let Some(bps) = slippage_bps {
            query.push(("slippageBps", bps.to_string()));
        }
        let value = self.get("/swap/v2/build", &query).await?;
        serde_json::from_value(value).map_err(|err| AppError::Internal(anyhow!("unexpected Jupiter build response: {err}")))
    }
}

fn to_instruction(ix: &JupInstruction) -> Result<Instruction> {
    let program_id = Pubkey::parse(&ix.program_id).ok_or_else(|| anyhow!("invalid program id from Jupiter"))?;
    let accounts = ix
        .accounts
        .iter()
        .map(|account| {
            Pubkey::parse(&account.pubkey)
                .map(|pubkey| AccountMeta {
                    pubkey,
                    is_signer: account.is_signer,
                    is_writable: account.is_writable,
                })
                .ok_or_else(|| anyhow!("invalid account from Jupiter"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Instruction {
        program_id,
        accounts,
        data: from_base64(&ix.data)?,
    })
}

pub fn swap_instructions(build: &JupBuild) -> Result<(Vec<Instruction>, Vec<LookupTable>)> {
    let mut instructions = Vec::new();
    for ix in build
        .compute_budget_instructions
        .iter()
        .chain(&build.setup_instructions)
        .chain(std::iter::once(&build.swap_instruction))
        .chain(build.cleanup_instruction.iter())
        .chain(&build.other_instructions)
    {
        instructions.push(to_instruction(ix)?);
    }
    let mut tables = Vec::new();
    for (key, addresses) in build.addresses_by_lookup_table_address.iter().flatten() {
        let key = Pubkey::parse(key).ok_or_else(|| anyhow!("invalid lookup table from Jupiter"))?;
        let addresses = addresses
            .iter()
            .map(|address| Pubkey::parse(address))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| anyhow!("invalid lookup table address from Jupiter"))?;
        tables.push(LookupTable { key, addresses });
    }
    Ok((instructions, tables))
}
