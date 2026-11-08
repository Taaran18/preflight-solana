export type RiskLevel = "ok" | "info" | "warn" | "danger";

export interface RiskFlag {
  level: RiskLevel;
  code: string;
  title: string;
  detail: string;
}

export type Verdict = "safe" | "caution" | "danger";

export interface TokenMeta {
  mint: string;
  symbol: string;
  name: string;
  decimals: number;
  icon: string | null;
  verified: boolean;
  tokenProgram: string;
  usdPrice: number | null;
}

export interface Holding extends TokenMeta {
  amount: number;
  raw: string;
  usdValue: number | null;
  native: boolean;
  account: string | null;
  accountRaw: string | null;
}

export interface Portfolio {
  owner: string;
  sol: number;
  holdings: Holding[];
  totalUsd: number;
  fetchedAt: number;
}

export type IntentAction = "swap" | "transfer" | "token_report" | "portfolio" | "unsupported";

export interface Intent {
  action: IntentAction;
  inputToken: string | null;
  outputToken: string | null;
  amountKind: "exact" | "percent" | "usd" | "all" | null;
  amountValue: number | null;
  amountOf: "input" | "output" | null;
  recipient: string | null;
  slippageBps: number | null;
  reply: string;
}

export interface BalanceChange {
  mint: string;
  symbol: string;
  icon: string | null;
  native: boolean;
  verified: boolean;
  before: number;
  after: number;
  delta: number;
  usdDelta: number | null;
}

export interface ProgramRef {
  id: string;
  name: string | null;
  topLevel: boolean;
}

export interface AuthorityEvent {
  kind: "owner_change" | "delegate" | "close" | "wallet_assigned" | "close_authority";
  mint: string;
  symbol: string;
  account: string;
  to: string | null;
  amount: number | null;
  unlimited: boolean;
}

export interface SimulationSummary {
  ok: boolean;
  error: string | null;
  unitsConsumed: number | null;
  changes: BalanceChange[];
  programs: ProgramRef[];
  events: AuthorityEvent[];
  logsTail: string[];
}

export interface Explanation {
  verdict: Verdict;
  headline: string;
  summary: string;
  points: string[];
  recommendation: string;
  model: string;
}

export interface QuoteInfo {
  inAmount: number;
  outAmount: number;
  minOut: number;
  priceImpactPct: number;
  slippageBps: number;
  route: string[];
  inUsd: number | null;
  outUsd: number | null;
}

export type ReceiptMode = "guard" | "memo" | "none";

export interface Receipt {
  hash: string;
  mode: ReceiptMode;
  program: string | null;
  memo: string | null;
  checks: string[];
  report: string;
}

export interface PlanResult {
  kind: "swap" | "transfer";
  owner: string;
  prompt: string;
  input: TokenMeta;
  output: TokenMeta | null;
  amount: number;
  amountNote: string | null;
  quote: QuoteInfo | null;
  recipient: string | null;
  simulation: SimulationSummary;
  risks: RiskFlag[];
  explanation: Explanation;
  receipt: Receipt;
  transaction: string;
  lastValidBlockHeight: number;
  expiresAt: number;
  feeSol: number;
  cuLimit: number;
}

export interface TokenStats {
  liquidityUsd: number | null;
  mcapUsd: number | null;
  holders: number | null;
  ageDays: number | null;
  organicScore: number | null;
  organicLabel: string | null;
  topHoldersPct: number | null;
  priceChange24h: number | null;
  program: string | null;
  extensions: string[];
  mintAuthority: string | null;
  freezeAuthority: string | null;
}

export interface TokenReport {
  token: TokenMeta;
  stats: TokenStats;
  risks: RiskFlag[];
  explanation: Explanation;
  lookalikes: number;
}

export interface InspectionResult {
  perspective: string;
  feePayer: string;
  perspectiveSigns: boolean;
  instructionCount: number;
  memos: string[];
  simulation: SimulationSummary;
  risks: RiskFlag[];
  explanation: Explanation;
}

export interface StageDef {
  id: string;
  label: string;
}

export type StageStatus = "idle" | "active" | "done" | "error" | "skipped";

export type StreamEvent =
  | { type: "stages"; stages: StageDef[] }
  | { type: "stage"; id: string; status: StageStatus; note?: string }
  | { type: "intent"; intent: Intent; parsedBy: string }
  | { type: "message"; text: string }
  | { type: "plan"; plan: PlanResult }
  | { type: "token_report"; report: TokenReport }
  | { type: "inspection"; result: InspectionResult }
  | { type: "portfolio"; portfolio: Portfolio }
  | { type: "notice"; text: string }
  | { type: "error"; message: string }
  | { type: "done" };

export interface Health {
  ok: boolean;
  ai: boolean;
  model: string;
  jupiterKey: boolean;
  guardProgram: string | null;
  rpc: string;
}

export interface SampleResponse {
  transaction: string;
  description: string;
}

export interface TamperResponse {
  guarded: boolean;
  reverted: boolean;
  headline: string;
  detail: string;
  logs: string[];
}

export interface ConfirmResponse {
  status: string;
  err: unknown;
  slot: number | null;
}

export interface MarketToken {
  symbol: string;
  mint: string;
  icon: string | null;
  usdPrice: number | null;
  change24h: number | null;
  verified: boolean;
}

export interface Market {
  slot: number;
  tps: number | null;
  slotTime: number | null;
  tokens: MarketToken[];
  updatedAt: number;
}

export interface Activity {
  id: string;
  label: string;
  signature: string;
  seconds: number;
  guarded: boolean;
}
