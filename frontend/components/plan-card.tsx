"use client";

import { useEffect, useState, type ReactNode } from "react";
import { VersionedTransaction } from "@solana/web3.js";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { base64ToBytes, getJson, postJson, sha256Hex } from "@/lib/api";
import { cx, explorerAccount, explorerTx, formatAmount, formatUsd, shortAddress } from "@/lib/format";
import type { Activity, ConfirmResponse, PlanResult, TamperResponse, TokenMeta } from "@/lib/types";
import { useToast } from "./toast";
import {
  Address,
  Analysis,
  BalanceChanges,
  Card,
  CopyButton,
  ErrorBox,
  Presence,
  RiskList,
  Section,
  Split,
  TokenAvatar,
  VERDICT,
  VerdictHeader,
  VerifiedBadge,
  buttonClass,
} from "./ui";
import {
  ArrowDownIcon,
  ArrowRightIcon,
  BoltIcon,
  ExternalIcon,
  LockIcon,
  RefreshIcon,
  ShieldCheckIcon,
  ShieldIcon,
  SparkIcon,
  XCircleIcon,
} from "./icons";

type Exec =
  | { phase: "idle" }
  | { phase: "signing" }
  | { phase: "confirming"; signature: string }
  | { phase: "confirmed"; signature: string; seconds: number }
  | { phase: "failed"; message: string; signature?: string };

const QUOTE_SECONDS = 55;

function describe(plan: PlanResult): string {
  if (plan.kind === "swap" && plan.quote && plan.output) {
    return `Swapped ${formatAmount(plan.quote.inAmount)} ${plan.input.symbol} → ${formatAmount(plan.quote.outAmount)} ${plan.output.symbol}`;
  }
  return `Sent ${formatAmount(plan.amount)} ${plan.input.symbol} to ${shortAddress(plan.recipient ?? "")}`;
}

function friendly(err: unknown): string {
  const message = err instanceof Error ? err.message : String(err);
  if (/reject|denied|cancel/i.test(message)) return "You cancelled the request in your wallet.";
  return message;
}

function TokenBox({
  label,
  token,
  amount,
  usd,
  sub,
}: {
  label: string;
  token: TokenMeta;
  amount: number;
  usd: number | null;
  sub?: string;
}) {
  return (
    <div className="rounded-xl border border-line bg-surface-2 p-4">
      <div className="text-[11px] font-medium uppercase tracking-[0.12em] text-faint">{label}</div>
      <div className="mt-2 flex items-center gap-3">
        <TokenAvatar icon={token.icon} symbol={token.symbol} size={38} />
        <div className="min-w-0">
          <div className="truncate font-mono text-xl font-semibold tracking-tight tabular-nums text-ink">{formatAmount(amount)}</div>
          <div className="flex items-center gap-1.5 text-sm text-mute">
            <span className="font-medium">{token.symbol}</span>
            {token.verified && <VerifiedBadge />}
            <span className="text-faint">·</span>
            <span className="font-mono text-xs tabular-nums">{formatUsd(usd)}</span>
          </div>
        </div>
      </div>
      {sub && <div className="mt-2 font-mono text-xs tabular-nums text-faint">{sub}</div>}
    </div>
  );
}

function Chip({ label, value, tone }: { label: string; value: string; tone?: "warn" | "bad" }) {
  return (
    <span className="inline-flex min-w-0 max-w-full items-center gap-1.5 rounded-lg border border-line bg-surface px-2.5 py-1.5">
      <span className="shrink-0 text-faint">{label}</span>
      <span
        className={cx(
          "truncate font-mono font-semibold tabular-nums",
          tone === "warn" && "text-warn",
          tone === "bad" && "text-bad",
          !tone && "text-ink",
        )}
      >
        {value}
      </span>
    </span>
  );
}

function FlowArrow() {
  return (
    <div className="mx-auto grid h-9 w-9 place-items-center rounded-full border border-line bg-surface text-mute shadow-sm">
      <ArrowRightIcon className="hidden h-4 w-4 sm:block" />
      <ArrowDownIcon className="h-4 w-4 sm:hidden" />
    </div>
  );
}

function TradeSummary({ plan }: { plan: PlanResult }) {
  const quote = plan.quote;
  if (plan.kind === "swap" && quote && plan.output) {
    const impactTone = quote.priceImpactPct >= 5 ? "bad" : quote.priceImpactPct >= 1 ? "warn" : undefined;
    return (
      <div className="space-y-3">
        <div className="grid items-center gap-2 sm:grid-cols-[1fr_auto_1fr]">
          <TokenBox label="You Pay" token={plan.input} amount={quote.inAmount} usd={quote.inUsd} />
          <FlowArrow />
          <TokenBox
            label="You Receive"
            token={plan.output}
            amount={quote.outAmount}
            usd={quote.outUsd}
            sub={`at least ${formatAmount(quote.minOut)} ${plan.output.symbol}`}
          />
        </div>
        <div className="flex flex-wrap gap-2 text-xs">
          <Chip label="Route" value={quote.route.length ? quote.route.join(" → ") : "Direct"} />
          <Chip label="Price Impact" value={`${quote.priceImpactPct.toFixed(2)}%`} tone={impactTone} />
          <Chip label="Max Slippage" value={`${quote.slippageBps / 100}%`} tone={quote.slippageBps > 300 ? "warn" : undefined} />
          <Chip label="Network Fee" value={`≈ ${formatAmount(plan.feeSol)} SOL`} />
        </div>
        {plan.amountNote && <p className="text-xs text-faint">{plan.amountNote}</p>}
      </div>
    );
  }
  return (
    <div className="space-y-3">
      <div className="grid items-center gap-2 sm:grid-cols-[1fr_auto_1fr]">
        <TokenBox
          label="You Send"
          token={plan.input}
          amount={plan.amount}
          usd={plan.input.usdPrice != null ? plan.input.usdPrice * plan.amount : null}
        />
        <FlowArrow />
        <div className="rounded-xl border border-line bg-surface-2 p-4">
          <div className="text-[11px] font-medium uppercase tracking-[0.12em] text-faint">Recipient</div>
          <div className="mt-2.5">{plan.recipient && <Address value={plan.recipient} href={explorerAccount(plan.recipient)} />}</div>
          <div className="mt-2 text-xs text-faint">Check the first and last characters.</div>
        </div>
      </div>
      <div className="flex flex-wrap gap-2 text-xs">
        <Chip label="Network Fee" value={`≈ ${formatAmount(plan.feeSol)} SOL`} />
        {plan.amountNote && <Chip label="Note" value={plan.amountNote} />}
      </div>
    </div>
  );
}

function ReceiptPanel({ plan }: { plan: PlanResult }) {
  const receipt = plan.receipt;
  const [verified, setVerified] = useState<boolean | null>(null);
  const [tamper, setTamper] = useState<TamperResponse | null>(null);
  const [tampering, setTampering] = useState(false);
  const [tamperError, setTamperError] = useState<string | null>(null);
  const [showReport, setShowReport] = useState(false);

  async function verify() {
    setVerified((await sha256Hex(receipt.report)) === receipt.hash);
  }

  async function runTamper() {
    setTampering(true);
    setTamperError(null);
    setTamper(null);
    try {
      setTamper(await postJson<TamperResponse>("/api/tamper", { transaction: plan.transaction, owner: plan.owner }));
    } catch (err) {
      setTamperError(err instanceof Error ? err.message : String(err));
    } finally {
      setTampering(false);
    }
  }

  let pretty = receipt.report;
  try {
    pretty = JSON.stringify(JSON.parse(receipt.report), null, 2);
  } catch {}

  return (
    <div className="space-y-3">
      {receipt.mode === "guard" ? (
        <>
          <p className="text-sm leading-relaxed text-mute">
            This preview is <span className="font-semibold text-ok">enforced on-chain</span>. A Rust program runs as the last
            step and reverts everything if your balances end up different from what you approved here.
          </p>
          <ul className="grid gap-1.5 rounded-xl border border-ok-line bg-ok-soft p-3">
            {receipt.checks.map((check, index) => (
              <li
                key={check}
                style={{ animationDelay: `${index * 60}ms` }}
                className="flex animate-enter items-start gap-2 text-sm text-ink"
              >
                <ShieldCheckIcon className="mt-0.5 h-4 w-4 shrink-0 text-ok" />
                {check}
              </li>
            ))}
          </ul>
          <div className="flex flex-wrap items-center gap-2">
            <button type="button" onClick={runTamper} disabled={tampering} className={buttonClass("secondary", "sm")}>
              {tampering ? <span className="spinner text-primary" /> : <ShieldIcon className="h-3.5 w-3.5" />}
              Try To Tamper With It
            </button>
            <span className="text-xs text-faint">Injects a hidden transfer and simulates it.</span>
          </div>
          {tamperError && <ErrorBox message={tamperError} />}
          <Presence show={tamper !== null}>
            {tamper && (
              <div className={cx("rounded-xl border px-3.5 py-3", tamper.reverted ? "border-ok-line bg-ok-soft" : "border-bad-line bg-bad-soft")}>
                <div className={cx("flex items-center gap-2 text-sm font-semibold", tamper.reverted ? "text-ok" : "text-bad")}>
                  {tamper.reverted ? <ShieldCheckIcon className="h-4 w-4" /> : <XCircleIcon className="h-4 w-4" />}
                  {tamper.headline}
                </div>
                <p className="mt-1 text-[13px] leading-relaxed text-mute">{tamper.detail}</p>
                {tamper.logs.length > 0 && (
                  <pre className="scroll-thin mt-2 max-h-32 overflow-auto rounded-lg border border-line bg-surface p-2 font-mono text-[10.5px] leading-relaxed text-mute">
                    {tamper.logs.join("\n")}
                  </pre>
                )}
              </div>
            )}
          </Presence>
        </>
      ) : receipt.mode === "memo" ? (
        <p className="text-sm leading-relaxed text-mute">
          A fingerprint of this exact analysis is written into the transaction memo, so anyone can later prove what you were
          shown before you signed. Deploy Preflight Guard to also make the preview binding.
        </p>
      ) : (
        <p className="text-sm leading-relaxed text-mute">
          This route was too large to carry an on-chain receipt, but the hash below still fingerprints the analysis.
        </p>
      )}
      <div className="flex flex-wrap items-center gap-2 rounded-xl border border-line bg-surface-2 px-3 py-2">
        <span className="text-xs text-faint">Report Hash</span>
        <code className="font-mono text-xs text-ink">
          {receipt.hash.slice(0, 14)}…{receipt.hash.slice(-8)}
        </code>
        <CopyButton value={receipt.hash} />
        <button
          type="button"
          onClick={verify}
          className={cx(
            "ml-auto rounded-md px-2 py-1 text-xs font-semibold transition",
            verified === null && "text-primary-ink hover:bg-primary-soft",
            verified === true && "animate-pop text-ok",
            verified === false && "animate-pop text-bad",
          )}
        >
          {verified === null ? "Verify Hash" : verified ? "✓ Matches The Report" : "✗ Does Not Match"}
        </button>
      </div>
      <button
        type="button"
        onClick={() => setShowReport((value) => !value)}
        className="text-xs font-medium text-faint transition hover:text-ink"
      >
        {showReport ? "Hide The Hashed Report" : "Show The Hashed Report"}
      </button>
      <Presence show={showReport}>
        <pre className="scroll-thin max-h-60 overflow-auto rounded-xl border border-line bg-surface-2 p-3 font-mono text-[11px] leading-relaxed text-mute">
          {pretty}
        </pre>
      </Presence>
    </div>
  );
}

function Countdown({ secondsLeft }: { secondsLeft: number }) {
  const radius = 9;
  const circumference = 2 * Math.PI * radius;
  const progress = Math.min(1, secondsLeft / QUOTE_SECONDS);
  const tone = secondsLeft === 0 ? "text-bad" : secondsLeft < 15 ? "text-warn" : "text-primary";
  return (
    <div className="flex items-center gap-2 text-xs text-mute">
      <svg viewBox="0 0 24 24" className={cx("h-6 w-6 -rotate-90", tone)}>
        <circle cx="12" cy="12" r={radius} fill="none" className="stroke-line-strong" strokeWidth="2.5" />
        <circle
          cx="12"
          cy="12"
          r={radius}
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={circumference * (1 - progress)}
          style={{ transition: "stroke-dashoffset 1s linear" }}
        />
      </svg>
      <span className="tabular-nums">{secondsLeft === 0 ? "Quote Expired" : `Quote Valid ${secondsLeft}s`}</span>
    </div>
  );
}

function SuccessMark() {
  return (
    <svg viewBox="0 0 24 24" className="check-draw h-5 w-5" fill="none" stroke="currentColor" strokeWidth={2.6} strokeLinecap="round" strokeLinejoin="round">
      <path d="M5 12.5l4.5 4.5L19 7.5" />
    </svg>
  );
}

export function PlanCard({
  plan,
  onExecuted,
  onRefresh,
}: {
  plan: PlanResult;
  onExecuted: (activity: Activity) => void;
  onRefresh: () => void;
}) {
  const { publicKey, sendTransaction } = useWallet();
  const { connection } = useConnection();
  const toast = useToast();
  const [exec, setExec] = useState<Exec>({ phase: "idle" });
  const [acknowledged, setAcknowledged] = useState(false);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  const verdict = plan.explanation.verdict;
  const secondsLeft = Math.max(0, Math.ceil((plan.expiresAt - now) / 1000));
  const expired = secondsLeft === 0;
  const isOwner = publicKey?.toBase58() === plan.owner;
  const busy = exec.phase === "signing" || exec.phase === "confirming";
  const confirmed = exec.phase === "confirmed";
  const needsAck = verdict === "danger" && plan.simulation.ok;

  async function execute() {
    if (!publicKey || busy) return;
    setExec({ phase: "signing" });
    let signature: string | undefined;
    try {
      const transaction = VersionedTransaction.deserialize(base64ToBytes(plan.transaction));
      const feePayer = transaction.message.staticAccountKeys[0];
      if (!feePayer || !feePayer.equals(publicKey) || plan.owner !== publicKey.toBase58()) {
        throw new Error("This transaction was built for a different wallet. Refresh the quote and try again.");
      }
      signature = await sendTransaction(transaction, connection, { maxRetries: 3 });
      const started = performance.now();
      setExec({ phase: "confirming", signature });
      const deadline = Date.now() + 60_000;
      while (Date.now() < deadline) {
        const status = await getJson<ConfirmResponse>(
          `/api/confirm?sig=${encodeURIComponent(signature)}&owner=${encodeURIComponent(plan.owner)}`,
        );
        if (status.err) throw new Error(`The transaction failed on-chain: ${JSON.stringify(status.err)}`);
        if (status.status === "confirmed" || status.status === "finalized") {
          const seconds = (performance.now() - started) / 1000;
          setExec({ phase: "confirmed", signature, seconds });
          toast({ title: `Confirmed in ${seconds.toFixed(2)}s`, detail: describe(plan), tone: "ok" });
          onExecuted({ id: signature, label: describe(plan), signature, seconds, guarded: plan.receipt.mode === "guard" });
          return;
        }
        await new Promise((resolve) => setTimeout(resolve, 400));
      }
      throw new Error("Not confirmed within 60 seconds. Check the explorer before trying again.");
    } catch (err) {
      const message = friendly(err);
      setExec({ phase: "failed", message, signature });
      toast({ title: "Transaction not sent", detail: message, tone: "bad" });
    }
  }

  let action: ReactNode;
  if (confirmed) {
    action = (
      <a href={explorerTx(exec.signature)} target="_blank" rel="noreferrer noopener" className={cx(buttonClass("secondary", "lg"), "w-full sm:w-auto")}>
        View On Solscan
        <ExternalIcon className="h-4 w-4" />
      </a>
    );
  } else if (expired && !busy) {
    action = (
      <button type="button" onClick={onRefresh} className={cx(buttonClass("primary", "lg"), "w-full sm:w-auto")}>
        <RefreshIcon className="h-4 w-4" />
        Refresh Quote
      </button>
    );
  } else {
    const disabledReason = !plan.simulation.ok
      ? "Simulation Failed"
      : !publicKey
        ? "Connect Wallet To Sign"
        : !isOwner
          ? `Switch To ${shortAddress(plan.owner)}`
          : needsAck && !acknowledged
            ? "Confirm The Risks First"
            : null;
    action = (
      <button
        type="button"
        onClick={execute}
        disabled={busy || disabledReason !== null}
        className={cx(buttonClass(verdict === "danger" ? "danger" : "primary", "lg"), "w-full sm:w-auto sm:min-w-44")}
      >
        {busy ? <span className="spinner" /> : <LockIcon className="h-4 w-4" />}
        {busy ? (exec.phase === "signing" ? "Approve In Wallet…" : "Confirming…") : (disabledReason ?? "Sign & Send")}
      </button>
    );
  }

  return (
    <Card className={cx("animate-enter overflow-hidden", VERDICT[verdict].line)}>
      <VerdictHeader
        explanation={plan.explanation}
        aside={
          plan.receipt.mode === "guard" ? (
            <span className="inline-flex items-center gap-1 rounded-md border border-ok-line bg-surface px-1.5 py-0.5 text-[10px] font-semibold text-ok">
              <ShieldCheckIcon className="h-3 w-3" />
              Guarded
            </span>
          ) : undefined
        }
      />
      <div className="px-4 py-5 sm:px-5">
        <TradeSummary plan={plan} />
      </div>
      <Section
        title="What Changes In Your Wallet"
        icon={<BoltIcon />}
        aside={
          plan.simulation.ok
            ? `Simulated On Mainnet${plan.simulation.unitsConsumed ? ` · ${plan.simulation.unitsConsumed.toLocaleString()} CU` : ""}`
            : "Simulation Failed"
        }
      >
        {plan.simulation.ok ? (
          <BalanceChanges changes={plan.simulation.changes} />
        ) : (
          <ErrorBox message={plan.simulation.error ?? "The simulation failed."} />
        )}
      </Section>
      <Split>
        <Section title="AI Analysis" icon={<SparkIcon />}>
          <Analysis explanation={plan.explanation} />
        </Section>
        <Section title="Safety Checks" icon={<ShieldIcon />} aside={`${plan.risks.length} Checks`}>
          <RiskList risks={plan.risks} />
        </Section>
      </Split>
      <Section title={plan.receipt.mode === "guard" ? "Preflight Guard" : "Verifiable Receipt"} icon={<LockIcon />}>
        <ReceiptPanel plan={plan} />
      </Section>
      <div className="border-t border-line bg-surface-2 px-4 py-4 sm:px-5">
        <Presence show={exec.phase === "confirming"}>
          {exec.phase === "confirming" && (
            <div className="mb-3 flex items-center gap-2 text-sm text-ink">
              <span className="spinner text-primary" />
              Confirming On Solana…
              <a
                href={explorerTx(exec.signature)}
                target="_blank"
                rel="noreferrer noopener"
                className="ml-1 font-mono text-xs text-mute hover:text-ink"
              >
                {shortAddress(exec.signature, 6)}
              </a>
            </div>
          )}
        </Presence>
        {exec.phase === "confirmed" && (
          <div className="mb-3 flex animate-pop items-center gap-2 rounded-xl border border-ok-line bg-ok-soft px-3.5 py-2.5 text-sm font-medium text-ok">
            <SuccessMark />
            Confirmed in <span className="font-mono font-semibold tabular-nums">{exec.seconds.toFixed(2)}s</span>
            <span className="font-normal text-mute">· Settled On Solana</span>
          </div>
        )}
        {exec.phase === "failed" && (
          <div className="mb-3">
            <ErrorBox message={exec.message} />
          </div>
        )}
        {needsAck && !confirmed && (
          <label className="mb-3 flex cursor-pointer items-start gap-2.5 rounded-xl border border-bad-line bg-bad-soft px-3.5 py-2.5 text-sm text-ink">
            <input
              type="checkbox"
              checked={acknowledged}
              onChange={(event) => setAcknowledged(event.target.checked)}
              className="mt-0.5 h-4 w-4 accent-red-600"
            />
            I&apos;ve read the red flags above and still want to sign this transaction.
          </label>
        )}
        <div className="flex flex-wrap items-center gap-x-4 gap-y-3">
          {!confirmed && <Countdown secondsLeft={secondsLeft} />}
          <span className="order-last flex basis-full items-center gap-1.5 text-xs text-faint sm:order-0 sm:basis-auto">
            <LockIcon className="h-3.5 w-3.5" />
            Non-Custodial · You Sign In Your Own Wallet
          </span>
          <div className="w-full sm:ml-auto sm:w-auto">{action}</div>
        </div>
      </div>
    </Card>
  );
}
