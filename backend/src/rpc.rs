use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};

use crate::solana::{Pubkey, from_base64};

pub struct Rpc {
    http: reqwest::Client,
    url: String,
    id: AtomicU64,
}

#[derive(Clone, Debug)]
pub struct RawAccount {
    pub lamports: u64,
    pub owner: Pubkey,
    pub data: Vec<u8>,
    pub executable: bool,
}

pub struct SimValue {
    pub err: Option<Value>,
    pub logs: Vec<String>,
    pub accounts: Option<Vec<Option<RawAccount>>>,
    pub units: Option<u64>,
}

pub struct TokenAccountInfo {
    pub address: Pubkey,
    pub mint: String,
    pub amount: u64,
    pub decimals: u8,
    pub program: Pubkey,
}

pub struct SignatureStatus {
    pub status: String,
    pub err: Option<Value>,
    pub slot: Option<u64>,
}

impl Rpc {
    pub fn new(http: reqwest::Client, url: String) -> Self {
        Self {
            http,
            url,
            id: AtomicU64::new(1),
        }
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        let body = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let response = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|err| anyhow!("Solana RPC {method} request failed: {}", err.without_url()))?;
        let status = response.status();
        if status.as_u16() == 429 {
            bail!("Solana RPC is rate-limiting requests. Set SOLANA_RPC_URL to a dedicated RPC (for example a free Helius key).");
        }
        let value: Value = response
            .json()
            .await
            .map_err(|err| anyhow!("Solana RPC {method} returned invalid JSON (HTTP {status}): {}", err.without_url()))?;
        if let Some(error) = value.get("error") {
            let message = error.get("message").and_then(Value::as_str).unwrap_or("unknown error");
            bail!("Solana RPC {method} failed: {message}");
        }
        Ok(value.get("result").cloned().unwrap_or(Value::Null))
    }

    pub fn parse_account(value: &Value) -> Option<RawAccount> {
        if value.is_null() {
            return None;
        }
        let data = value.get("data")?.get(0)?.as_str()?;
        Some(RawAccount {
            lamports: value.get("lamports")?.as_u64()?,
            owner: Pubkey::parse(value.get("owner")?.as_str()?)?,
            data: from_base64(data).ok()?,
            executable: value.get("executable").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    pub async fn balance(&self, owner: &Pubkey) -> Result<u64> {
        let result = self.call("getBalance", json!([owner.to_string(), { "commitment": "confirmed" }])).await?;
        result
            .get("value")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("unexpected getBalance response"))
    }

    pub async fn account(&self, address: &Pubkey) -> Result<Option<RawAccount>> {
        let result = self
            .call(
                "getAccountInfo",
                json!([address.to_string(), { "encoding": "base64", "commitment": "confirmed" }]),
            )
            .await?;
        Ok(result.get("value").and_then(Self::parse_account))
    }

    pub async fn accounts(&self, addresses: &[Pubkey]) -> Result<Vec<Option<RawAccount>>> {
        let mut out = Vec::with_capacity(addresses.len());
        for chunk in addresses.chunks(100) {
            let keys: Vec<String> = chunk.iter().map(Pubkey::to_string).collect();
            let result = self
                .call(
                    "getMultipleAccounts",
                    json!([keys, { "encoding": "base64", "commitment": "confirmed" }]),
                )
                .await?;
            let values = result
                .get("value")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("unexpected getMultipleAccounts response"))?;
            out.extend(values.iter().map(Self::parse_account));
        }
        Ok(out)
    }

    pub async fn token_accounts(&self, owner: &Pubkey, program: &Pubkey) -> Result<Vec<TokenAccountInfo>> {
        let result = self
            .call(
                "getTokenAccountsByOwner",
                json!([
                    owner.to_string(),
                    { "programId": program.to_string() },
                    { "encoding": "jsonParsed", "commitment": "confirmed" }
                ]),
            )
            .await?;
        let items = result.get("value").and_then(Value::as_array).cloned().unwrap_or_default();
        Ok(items
            .iter()
            .filter_map(|item| {
                let address = Pubkey::parse(item.get("pubkey")?.as_str()?)?;
                let info = item.pointer("/account/data/parsed/info")?;
                let amount = info.pointer("/tokenAmount/amount")?.as_str()?.parse().ok()?;
                let decimals = info.pointer("/tokenAmount/decimals")?.as_u64()? as u8;
                Some(TokenAccountInfo {
                    address,
                    mint: info.get("mint")?.as_str()?.to_string(),
                    amount,
                    decimals,
                    program: *program,
                })
            })
            .collect())
    }

    pub async fn latest_blockhash(&self) -> Result<([u8; 32], u64)> {
        let result = self.call("getLatestBlockhash", json!([{ "commitment": "confirmed" }])).await?;
        let hash = result
            .pointer("/value/blockhash")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("unexpected getLatestBlockhash response"))?;
        let height = result
            .pointer("/value/lastValidBlockHeight")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let bytes = bs58::decode(hash).into_vec()?;
        let blockhash: [u8; 32] = bytes.try_into().map_err(|_| anyhow!("invalid blockhash"))?;
        Ok((blockhash, height))
    }

    pub async fn simulate(&self, transaction: &str, watch: &[Pubkey]) -> Result<SimValue> {
        let mut config = json!({
            "encoding": "base64",
            "sigVerify": false,
            "replaceRecentBlockhash": true,
            "commitment": "confirmed"
        });
        if !watch.is_empty() {
            let addresses: Vec<String> = watch.iter().map(Pubkey::to_string).collect();
            config["accounts"] = json!({ "encoding": "base64", "addresses": addresses });
        }
        let result = self.call("simulateTransaction", json!([transaction, config])).await?;
        let value = result.get("value").cloned().unwrap_or(Value::Null);
        let err = value.get("err").cloned().filter(|err| !err.is_null());
        let logs = value
            .get("logs")
            .and_then(Value::as_array)
            .map(|lines| lines.iter().filter_map(|line| line.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let accounts = value
            .get("accounts")
            .and_then(Value::as_array)
            .map(|items| items.iter().map(Self::parse_account).collect());
        let units = value.get("unitsConsumed").and_then(Value::as_u64);
        Ok(SimValue {
            err,
            logs,
            accounts,
            units,
        })
    }

    pub async fn signature_status(&self, signature: &str) -> Result<SignatureStatus> {
        let result = self
            .call(
                "getSignatureStatuses",
                json!([[signature], { "searchTransactionHistory": false }]),
            )
            .await?;
        let status = result.pointer("/value/0").cloned().unwrap_or(Value::Null);
        if status.is_null() {
            return Ok(SignatureStatus {
                status: "pending".to_string(),
                err: None,
                slot: None,
            });
        }
        Ok(SignatureStatus {
            status: status
                .get("confirmationStatus")
                .and_then(Value::as_str)
                .unwrap_or("processed")
                .to_string(),
            err: status.get("err").cloned().filter(|err| !err.is_null()),
            slot: status.get("slot").and_then(Value::as_u64),
        })
    }

    pub async fn network(&self) -> Result<(u64, Option<f64>, Option<f64>)> {
        let (slot, samples) = tokio::join!(
            self.call("getSlot", json!([{ "commitment": "confirmed" }])),
            self.call("getRecentPerformanceSamples", json!([4])),
        );
        let slot = slot?.as_u64().unwrap_or(0);
        let samples = samples.ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let (mut txs, mut slots, mut secs) = (0f64, 0f64, 0f64);
        for sample in &samples {
            txs += sample.get("numTransactions").and_then(Value::as_f64).unwrap_or(0.0);
            slots += sample.get("numSlots").and_then(Value::as_f64).unwrap_or(0.0);
            secs += sample.get("samplePeriodSecs").and_then(Value::as_f64).unwrap_or(0.0);
        }
        let tps = (secs > 0.0).then(|| txs / secs);
        let slot_time = (slots > 0.0).then(|| secs / slots);
        Ok((slot, tps, slot_time))
    }

    pub async fn recent_signature_count(&self, address: &Pubkey, limit: usize) -> Result<usize> {
        let result = self
            .call(
                "getSignaturesForAddress",
                json!([address.to_string(), { "limit": limit, "commitment": "confirmed" }]),
            )
            .await?;
        Ok(result.as_array().map(Vec::len).unwrap_or(0))
    }
}
