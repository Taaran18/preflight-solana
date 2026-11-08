"use client";

import { useEffect, useState } from "react";
import { DEMO_WALLET, getJson } from "@/lib/api";
import { cx, explorerTx, formatAmount, formatUsd, isAddress, shortAddress } from "@/lib/format";
import type { Activity, Market, Portfolio } from "@/lib/types";
import { Card, Skeleton, TokenAvatar, VerifiedBadge, buttonClass } from "./ui";
import {
  ActivityIcon,
  CheckIcon,
  CloseIcon,
  ExternalIcon,
  RefreshIcon,
  ShieldCheckIcon,
  TrendDownIcon,
  TrendUpIcon,
  WalletIcon,
} from "./icons";

export function MarketPanel({ onAsk }: { onAsk: (text: string) => void }) {
  const [market, setMarket] = useState<Market | null>(null);
  const [history, setHistory] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());

  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;
    async function load() {
      let failed = false;
      if (document.visibilityState === "visible") {
        try {
          const value = await getJson<Market>("/api/market");
          if (cancelled) return;
          setMarket(value);
          setError(null);
          if (value.tps != null) setHistory((list) => [...list.slice(-15), value.tps as number]);
        } catch (err) {
          if (cancelled) return;
          failed = true;
          setError(err instanceof Error ? err.message : String(err));
        }
      }
      timer = window.setTimeout(load, failed ? 5_000 : 20_000);
    }
    void load();
    const tick = window.setInterval(() => setNow(Date.now()), 5_000);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      window.clearInterval(tick);
    };
  }, [attempt]);

  const peak = Math.max(1, ...history);
  const seconds = market ? Math.max(0, Math.round((now - market.updatedAt) / 1000)) : null;

  return (
    <Card className="flex shrink-0 animate-enter flex-col overflow-hidden p-4 lg:grow">
      <div className="flex items-center gap-2">
        <span className="grid h-7 w-7 place-items-center rounded-lg bg-primary-soft text-primary">
          <ActivityIcon className="h-4 w-4" />
        </span>
        <span className="text-sm font-semibold text-ink">Solana Pulse</span>
        {error && !market ? (
          <span className="ml-auto inline-flex items-center gap-1.5 rounded-md border border-warn-line bg-warn-soft px-1.5 py-0.5 text-[10px] font-semibold text-warn">
            Unavailable
          </span>
        ) : (
          <span className="ml-auto inline-flex items-center gap-1.5 rounded-md border border-ok-line bg-ok-soft px-1.5 py-0.5 text-[10px] font-semibold text-ok">
            <span className="pulse-ring h-1.5 w-1.5 rounded-full bg-ok-solid text-ok-solid" />
            Live
          </span>
        )}
      </div>
      {error && !market && (
        <p className="mt-3 animate-fade rounded-xl border border-warn-line bg-warn-soft px-3 py-2 text-xs leading-relaxed text-ink">
          Live network data isn&apos;t reachable right now. Retrying automatically…
          <button
            type="button"
            onClick={() => setAttempt((value) => value + 1)}
            className="ml-1 font-semibold text-primary-ink underline-offset-2 hover:underline"
          >
            Retry Now
          </button>
        </p>
      )}

      <div className="mt-3 grid grid-cols-3 gap-2">
        {[
          { label: "TPS", value: market?.tps != null ? Math.round(market.tps).toLocaleString() : null },
          { label: "Block Time", value: market?.slotTime != null ? `${Math.round(market.slotTime * 1000)}ms` : null },
          { label: "Slot", value: market ? `${(market.slot / 1e6).toFixed(2)}M` : null },
        ].map((stat) => (
          <div key={stat.label} className="rounded-xl border border-line bg-surface-2 px-2.5 py-2">
            <div className="text-[10px] font-medium uppercase tracking-widest text-faint">{stat.label}</div>
            {stat.value ? (
              <div key={stat.value} className="mt-0.5 animate-fade font-mono text-sm font-semibold tabular-nums text-ink">
                {stat.value}
              </div>
            ) : error ? (
              <div className="mt-0.5 font-mono text-sm text-faint">–</div>
            ) : (
              <Skeleton className="mt-1.5 h-3.5 w-12" />
            )}
          </div>
        ))}
      </div>

      <div className="mt-3 flex h-12 items-end gap-1 rounded-xl border border-line bg-surface-2 px-2 pb-1.5 pt-2" aria-label="Recent TPS">
        {history.length === 0
          ? Array.from({ length: 16 }).map((_, index) => (
              <span key={index} className={cx("h-2 flex-1 rounded-sm", error ? "bg-surface-3" : "skeleton")} />
            ))
          : history.map((value, index) => (
              <span
                key={index}
                style={{ height: `${Math.max(12, (value / peak) * 100)}%` }}
                className={cx("flex-1 rounded-sm transition-all duration-700", index === history.length - 1 ? "bg-primary" : "bg-primary/35")}
              />
            ))}
      </div>

      <div className="mt-3 text-[11px] font-semibold uppercase tracking-[0.12em] text-faint">Markets</div>
      <div className="mt-1.5 flex-1 space-y-0.5">
        {market
          ? market.tokens.map((token, index) => {
              const up = (token.change24h ?? 0) >= 0;
              return (
                <button
                  key={token.mint}
                  type="button"
                  onClick={() => onAsk(token.symbol === "SOL" ? "Is SOL safe?" : `Is ${token.mint} safe?`)}
                  title={`Run A Safety Check On ${token.symbol}`}
                  style={{ animationDelay: `${index * 40}ms` }}
                  className="group flex w-full animate-enter items-center gap-2.5 rounded-lg px-1.5 py-1.5 text-left transition hover:bg-surface-2"
                >
                  <TokenAvatar icon={token.icon} symbol={token.symbol} size={24} />
                  <span className="flex-1 truncate text-sm font-medium text-ink">{token.symbol}</span>
                  <span className="font-mono text-xs tabular-nums text-ink">{formatUsd(token.usdPrice)}</span>
                  {token.change24h != null && (
                    <span
                      className={cx(
                        "inline-flex w-16 items-center justify-end gap-0.5 font-mono text-[11px] font-semibold tabular-nums",
                        up ? "text-ok" : "text-bad",
                      )}
                    >
                      {up ? <TrendUpIcon className="h-3 w-3" /> : <TrendDownIcon className="h-3 w-3" />}
                      {Math.abs(token.change24h).toFixed(1)}%
                    </span>
                  )}
                </button>
              );
            })
          : error ? null : [0, 1, 2, 3].map((row) => (
              <div key={row} className="flex items-center gap-2.5 px-1.5 py-1.5">
                <Skeleton className="h-6 w-6 rounded-full" />
                <Skeleton className="h-3 flex-1" />
                <Skeleton className="h-3 w-14" />
              </div>
            ))}
      </div>

      <div className="mt-2 flex items-center justify-between border-t border-line pt-2 text-[11px] text-faint">
        <span>Helius RPC · Jupiter</span>
        <span className={cx(error && "text-bad")}>{error ? "Offline" : seconds != null ? `Updated ${seconds}s Ago` : "Loading…"}</span>
      </div>
    </Card>
  );
}

export function PortfolioPanel({
  owner,
  connected,
  portfolio,
  loading,
  error,
  watch,
  onWatch,
  onRefresh,
  onAsk,
  onDemo,
}: {
  owner: string | null;
  connected: boolean;
  portfolio: Portfolio | null;
  loading: boolean;
  error: string | null;
  watch: string;
  onWatch: (value: string) => void;
  onRefresh: () => void;
  onAsk: (text: string) => void;
  onDemo: () => void;
}) {
  const holdings = portfolio?.holdings ?? [];
  const watchInvalid = watch.trim().length > 0 && !isAddress(watch);
  const isDemo = !connected && owner === DEMO_WALLET;

  return (
    <Card className="animate-enter overflow-hidden lg:shrink-0">
      <div className="flex items-center gap-2 px-4 pt-4">
        <span className="grid h-7 w-7 place-items-center rounded-lg bg-primary-soft text-primary">
          <WalletIcon className="h-4 w-4" />
        </span>
        <span className="text-sm font-semibold text-ink">Wallet</span>
        {owner && (
          <span
            className={cx(
              "animate-pop rounded-md border px-1.5 py-0.5 text-[10px] font-semibold",
              connected ? "border-ok-line bg-ok-soft text-ok" : "border-warn-line bg-warn-soft text-warn",
            )}
          >
            {connected ? "Connected" : isDemo ? "Demo · View Only" : "View Only"}
          </span>
        )}
        {owner && (
          <button
            type="button"
            onClick={onRefresh}
            title="Refresh balances"
            aria-label="Refresh balances"
            className="ml-auto rounded-lg p-1.5 text-mute transition hover:bg-surface-2 hover:text-ink"
          >
            <RefreshIcon className={cx("h-4 w-4", loading && "animate-spin")} />
          </button>
        )}
      </div>

      {!connected && (
        <div className="px-4 pt-3">
          {!owner && (
            <p className="text-[13px] leading-relaxed text-mute">Connect a wallet to trade, or preview everything as any address:</p>
          )}
          <div className="relative mt-2">
            <input
              value={watch}
              onChange={(event) => onWatch(event.target.value)}
              placeholder="Paste a Solana address"
              spellCheck={false}
              aria-label="Solana address to view"
              className={cx(
                "h-10 w-full rounded-xl border bg-surface-2 px-3 pr-8 font-mono text-base text-ink outline-none transition placeholder:font-sans placeholder:text-faint focus:ring-4 focus:ring-primary/12 sm:text-xs",
                watchInvalid ? "border-bad" : "border-line focus:border-primary/50",
              )}
            />
            {watch && (
              <button
                type="button"
                aria-label="Clear address"
                onClick={() => onWatch("")}
                className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-faint transition hover:text-ink"
              >
                <CloseIcon className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
          {watchInvalid && <p className="mt-1.5 animate-fade text-xs text-bad">That isn&apos;t a valid Solana address.</p>}
          {!owner && (
            <button type="button" onClick={onDemo} className={cx(buttonClass("secondary"), "mt-2 w-full")}>
              <WalletIcon className="h-4 w-4 text-primary" />
              Use The Demo Wallet
            </button>
          )}
          {isDemo && (
            <p className="mt-2 animate-fade text-xs leading-relaxed text-faint">
              A large public exchange wallet. Real balances and simulations, but nothing can be signed.
            </p>
          )}
        </div>
      )}

      {owner && (
        <div className="px-4 pb-1 pt-4">
          <div className="text-[11px] font-medium uppercase tracking-[0.12em] text-faint">Total Value</div>
          <div className="mt-1 flex items-baseline gap-2">
            {portfolio ? (
              <span key={portfolio.fetchedAt} className="animate-fade font-mono text-2xl font-semibold tracking-tight tabular-nums text-ink">
                {formatUsd(portfolio.totalUsd)}
              </span>
            ) : loading ? (
              <Skeleton className="h-7 w-32" />
            ) : (
              <span className="font-mono text-2xl font-semibold text-faint">–</span>
            )}
            <span className="font-mono text-xs text-faint">{shortAddress(owner)}</span>
          </div>
        </div>
      )}

      {error && <p className="animate-fade px-4 pt-2 text-xs text-bad">{error}</p>}

      {owner && loading && holdings.length === 0 && (
        <div className="space-y-3 px-4 py-3">
          {[0, 1, 2, 3].map((row) => (
            <div key={row} className="flex items-center gap-3">
              <Skeleton className="h-8 w-8 rounded-full" />
              <div className="flex-1 space-y-1.5">
                <Skeleton className="h-3 w-20" />
                <Skeleton className="h-2.5 w-14" />
              </div>
              <Skeleton className="h-3 w-12" />
            </div>
          ))}
        </div>
      )}

      {holdings.length > 0 && (
        <div className="scroll-thin mt-2 max-h-80 overflow-y-auto px-2 pb-2 lg:max-h-56">
          {holdings.slice(0, 20).map((holding, index) => (
            <button
              key={`${holding.mint}-${holding.native}`}
              type="button"
              onClick={() => onAsk(holding.native ? "Is SOL safe?" : `Is ${holding.mint} safe?`)}
              title={`Run a safety check on ${holding.symbol}`}
              style={{ animationDelay: `${Math.min(index, 10) * 35}ms` }}
              className="group flex w-full animate-enter items-center gap-3 rounded-xl px-2 py-2 text-left transition hover:bg-surface-2"
            >
              <TokenAvatar icon={holding.icon} symbol={holding.symbol} size={32} />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-1.5 text-sm font-medium text-ink">
                  <span className="truncate">{holding.symbol}</span>
                  {holding.verified && <VerifiedBadge />}
                </div>
                <div className="truncate font-mono text-xs tabular-nums text-faint">{formatAmount(holding.amount)}</div>
              </div>
              <div className="text-right">
                <div className="font-mono text-sm tabular-nums text-ink">{formatUsd(holding.usdValue)}</div>
                <div className="text-[10px] font-medium text-primary-ink opacity-0 transition group-hover:opacity-100">Check →</div>
              </div>
            </button>
          ))}
        </div>
      )}
      {!owner && <div className="h-4" />}
      {owner && holdings.length === 0 && !loading && !error && <p className="px-4 pb-4 pt-2 text-sm text-mute">No balances yet.</p>}
    </Card>
  );
}

export function ActivityPanel({ activity }: { activity: Activity[] }) {
  if (activity.length === 0) return null;
  return (
    <Card className="animate-enter p-4">
      <div className="text-sm font-semibold text-ink">This Session</div>
      <div className="mt-3 space-y-2.5">
        {activity.map((item) => (
          <a
            key={item.id}
            href={explorerTx(item.signature)}
            target="_blank"
            rel="noreferrer noopener"
            className="group flex animate-enter items-start gap-3 rounded-xl border border-line bg-surface-2 p-3 transition hover:border-line-strong"
          >
            <span className="mt-0.5 grid h-6 w-6 shrink-0 place-items-center rounded-full bg-ok-soft text-ok">
              <CheckIcon className="h-3.5 w-3.5" strokeWidth={3} />
            </span>
            <div className="min-w-0 flex-1">
              <div className="truncate text-sm text-ink">{item.label}</div>
              <div className="mt-0.5 flex items-center gap-2 text-xs text-faint">
                <span className="font-mono tabular-nums">{item.seconds.toFixed(2)}s</span>
                {item.guarded && (
                  <span className="inline-flex items-center gap-1 text-ok">
                    <ShieldCheckIcon className="h-3 w-3" />
                    Guarded
                  </span>
                )}
              </div>
            </div>
            <ExternalIcon className="h-3.5 w-3.5 text-faint transition group-hover:text-ink" />
          </a>
        ))}
      </div>
    </Card>
  );
}

const STEPS = [
  { title: "Understand", text: "Claude turns your request into a precise action." },
  { title: "Route", text: "Jupiter finds the best path across Solana exchanges." },
  { title: "Simulate", text: "The exact transaction runs against live mainnet state." },
  { title: "Explain", text: "The risk engine and AI flag anything that could hurt you." },
  { title: "Guard", text: "A Rust program enforces the preview on-chain." },
];

export function HowItWorks() {
  return (
    <Card className="animate-enter p-4 lg:shrink-0">
      <div className="text-sm font-semibold text-ink">How Preflight Protects You</div>
      <ol className="relative mt-3 space-y-2.5 before:absolute before:bottom-3 before:left-3 before:top-3 before:w-px before:bg-line">
        {STEPS.map((step, index) => (
          <li key={step.title} className="relative flex gap-3">
            <span className="relative grid h-6 w-6 shrink-0 place-items-center rounded-full border border-line bg-surface font-mono text-[11px] font-semibold text-primary-ink">
              {index + 1}
            </span>
            <div>
              <div className="text-sm font-medium text-ink">{step.title}</div>
              <div className="text-xs leading-relaxed text-mute">{step.text}</div>
            </div>
          </li>
        ))}
      </ol>
    </Card>
  );
}
