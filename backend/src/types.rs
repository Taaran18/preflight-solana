use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Ok,
    Info,
    Warn,
    Danger,
}

#[derive(Serialize, Clone, Debug)]
pub struct RiskFlag {
    pub level: RiskLevel,
    pub code: String,
    pub title: String,
    pub detail: String,
}

impl RiskFlag {
    pub fn new(level: RiskLevel, code: &str, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            level,
            code: code.to_string(),
            title: title.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Safe,
    Caution,
    Danger,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TokenMeta {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub icon: Option<String>,
    pub verified: bool,
    pub token_program: String,
    pub usd_price: Option<f64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Holding {
    #[serde(flatten)]
    pub meta: TokenMeta,
    pub amount: f64,
    pub raw: String,
    pub usd_value: Option<f64>,
    pub native: bool,
    pub account: Option<String>,
    pub account_raw: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Portfolio {
    pub owner: String,
    pub sol: f64,
    pub holdings: Vec<Holding>,
    pub total_usd: f64,
    pub fetched_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentAction {
    Swap,
    Transfer,
    TokenReport,
    Portfolio,
    Unsupported,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AmountKind {
    Exact,
    Percent,
    Usd,
    All,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AmountOf {
    Input,
    Output,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Intent {
    pub action: IntentAction,
    pub input_token: Option<String>,
    pub output_token: Option<String>,
    pub amount_kind: Option<AmountKind>,
    pub amount_value: Option<f64>,
    pub amount_of: Option<AmountOf>,
    pub recipient: Option<String>,
    pub slippage_bps: Option<u32>,
    pub reply: String,
}

impl Intent {
    pub fn reply(action: IntentAction, reply: impl Into<String>) -> Self {
        Self {
            action,
            input_token: None,
            output_token: None,
            amount_kind: None,
            amount_value: None,
            amount_of: None,
            recipient: None,
            slippage_bps: None,
            reply: reply.into(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BalanceChange {
    pub mint: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub native: bool,
    pub verified: bool,
    pub before: f64,
    pub after: f64,
    pub delta: f64,
    pub usd_delta: Option<f64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProgramRef {
    pub id: String,
    pub name: Option<String>,
    pub top_level: bool,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityKind {
    OwnerChange,
    WalletAssigned,
    CloseAuthority,
    Delegate,
    Close,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthorityEvent {
    pub kind: AuthorityKind,
    pub mint: String,
    pub symbol: String,
    pub account: String,
    pub to: Option<String>,
    pub amount: Option<f64>,
    pub unlimited: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SimulationSummary {
    pub ok: bool,
    pub error: Option<String>,
    pub units_consumed: Option<u64>,
    pub changes: Vec<BalanceChange>,
    pub programs: Vec<ProgramRef>,
    pub events: Vec<AuthorityEvent>,
    pub logs_tail: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub verdict: Verdict,
    pub headline: String,
    pub summary: String,
    pub points: Vec<String>,
    pub recommendation: String,
    pub model: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuoteInfo {
    pub in_amount: f64,
    pub out_amount: f64,
    pub min_out: f64,
    pub price_impact_pct: f64,
    pub slippage_bps: u32,
    pub route: Vec<String>,
    pub in_usd: Option<f64>,
    pub out_usd: Option<f64>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReceiptMode {
    Guard,
    Memo,
    None,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub hash: String,
    pub mode: ReceiptMode,
    pub program: Option<String>,
    pub memo: Option<String>,
    pub checks: Vec<String>,
    pub report: String,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlanKind {
    Swap,
    Transfer,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlanResult {
    pub kind: PlanKind,
    pub owner: String,
    pub prompt: String,
    pub input: TokenMeta,
    pub output: Option<TokenMeta>,
    pub amount: f64,
    pub amount_note: Option<String>,
    pub quote: Option<QuoteInfo>,
    pub recipient: Option<String>,
    pub simulation: SimulationSummary,
    pub risks: Vec<RiskFlag>,
    pub explanation: Explanation,
    pub receipt: Receipt,
    pub transaction: String,
    pub last_valid_block_height: u64,
    pub expires_at: u64,
    pub fee_sol: f64,
    pub cu_limit: u32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TokenStats {
    pub liquidity_usd: Option<f64>,
    pub mcap_usd: Option<f64>,
    pub holders: Option<f64>,
    pub age_days: Option<f64>,
    pub organic_score: Option<f64>,
    pub organic_label: Option<String>,
    pub top_holders_pct: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub program: Option<String>,
    pub extensions: Vec<String>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TokenReport {
    pub token: TokenMeta,
    pub stats: TokenStats,
    pub risks: Vec<RiskFlag>,
    pub explanation: Explanation,
    pub lookalikes: usize,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InspectionResult {
    pub perspective: String,
    pub fee_payer: String,
    pub perspective_signs: bool,
    pub instruction_count: usize,
    pub memos: Vec<String>,
    pub simulation: SimulationSummary,
    pub risks: Vec<RiskFlag>,
    pub explanation: Explanation,
}

#[derive(Serialize, Clone, Debug)]
pub struct StageDef {
    pub id: String,
    pub label: String,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    Active,
    Done,
    Error,
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum StreamEvent {
    Stages {
        stages: Vec<StageDef>,
    },
    Stage {
        id: String,
        status: StageStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    Intent {
        intent: Intent,
        parsed_by: String,
    },
    Message {
        text: String,
    },
    Plan {
        plan: Box<PlanResult>,
    },
    TokenReport {
        report: Box<TokenReport>,
    },
    Inspection {
        result: Box<InspectionResult>,
    },
    Portfolio {
        portfolio: Portfolio,
    },
    Notice {
        text: String,
    },
    Error {
        message: String,
    },
    Done,
}

#[derive(Deserialize)]
pub struct PlanRequest {
    pub prompt: String,
    pub owner: Option<String>,
}

#[derive(Deserialize)]
pub struct InspectRequest {
    pub transaction: String,
    pub owner: Option<String>,
}

#[derive(Deserialize)]
pub struct OwnerRequest {
    pub owner: String,
}

#[derive(Deserialize)]
pub struct TamperRequest {
    pub transaction: String,
    pub owner: String,
}

#[derive(Deserialize)]
pub struct OwnerQuery {
    pub owner: String,
    pub refresh: Option<bool>,
}

#[derive(Deserialize)]
pub struct SignatureQuery {
    pub sig: String,
    pub owner: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleResponse {
    pub transaction: String,
    pub description: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TamperResponse {
    pub guarded: bool,
    pub reverted: bool,
    pub headline: String,
    pub detail: String,
    pub logs: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketToken {
    pub symbol: String,
    pub mint: String,
    pub icon: Option<String>,
    pub usd_price: Option<f64>,
    pub change_24h: Option<f64>,
    pub verified: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarketResponse {
    pub slot: u64,
    pub tps: Option<f64>,
    pub slot_time: Option<f64>,
    pub tokens: Vec<MarketToken>,
    pub updated_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmResponse {
    pub status: String,
    pub err: Option<Value>,
    pub slot: Option<u64>,
}
