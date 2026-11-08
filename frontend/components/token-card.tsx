"use client";

import { cx, explorerToken, formatAge, formatCompact, formatCompactUsd, formatPercent, formatUsd } from "@/lib/format";
import type { TokenReport } from "@/lib/types";
import {
  Address,
  Analysis,
  Card,
  RiskList,
  Section,
  Split,
  Stat,
  TokenAvatar,
  VERDICT,
  VerdictHeader,
  VerifiedBadge,
  buttonClass,
} from "./ui";
import { BoltIcon, ExternalIcon, ShieldIcon, SparkIcon, SwapIcon, TrendDownIcon, TrendUpIcon } from "./icons";

export function TokenCard({ report, onAsk }: { report: TokenReport; onAsk: (text: string) => void }) {
  const { token, stats } = report;
  const change = stats.priceChange24h;
  const holdersTone = stats.topHoldersPct != null && stats.topHoldersPct > 50 ? "warn" : undefined;
  const liquidityTone =
    stats.liquidityUsd == null ? undefined : stats.liquidityUsd < 10_000 ? "bad" : stats.liquidityUsd < 100_000 ? "warn" : "good";

  return (
    <Card className={cx("animate-enter overflow-hidden", VERDICT[report.explanation.verdict].line)}>
      <VerdictHeader explanation={report.explanation} />
      <div className="flex items-center gap-3.5 px-4 pb-3 pt-5 sm:px-5">
        <TokenAvatar icon={token.icon} symbol={token.symbol} size={48} />
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-lg font-semibold tracking-tight text-ink">
            {token.symbol}
            {token.verified && <VerifiedBadge />}
          </div>
          <div className="truncate text-sm text-mute">{token.name}</div>
        </div>
        <div className="ml-auto text-right">
          <div className="font-mono text-lg font-semibold tabular-nums text-ink">{formatUsd(token.usdPrice)}</div>
          {change != null && (
            <div
              className={cx(
                "mt-0.5 inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 font-mono text-xs font-semibold tabular-nums",
                change >= 0 ? "bg-ok-soft text-ok" : "bg-bad-soft text-bad",
              )}
            >
              {change >= 0 ? <TrendUpIcon className="h-3 w-3" /> : <TrendDownIcon className="h-3 w-3" />}
              {change >= 0 ? "+" : ""}
              {change.toFixed(2)}% 24h
            </div>
          )}
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2 px-4 pb-4 text-xs text-faint sm:px-5">
        Mint
        <Address value={token.mint} href={explorerToken(token.mint)} />
        {stats.program && <span className="rounded-md border border-line bg-surface-2 px-1.5 py-0.5 text-[11px] text-mute">{stats.program}</span>}
        {report.lookalikes > 0 && (
          <span className="rounded-md border border-warn-line bg-warn-soft px-1.5 py-0.5 text-[11px] font-medium text-warn">
            {report.lookalikes} Look-Alike Tickers
          </span>
        )}
      </div>
      <Section title="Market & Authorities" icon={<BoltIcon />}>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          <Stat label="Liquidity" value={formatCompactUsd(stats.liquidityUsd)} tone={liquidityTone} />
          <Stat label="Market Cap" value={formatCompactUsd(stats.mcapUsd)} />
          <Stat label="Holders" value={formatCompact(stats.holders)} />
          <Stat label="Market Age" value={formatAge(stats.ageDays)} />
          <Stat label="Organic Score" value={stats.organicLabel ?? "–"} tone={stats.organicLabel === "low" ? "warn" : undefined} />
          <Stat label="Top Holders" value={formatPercent(stats.topHoldersPct, 0)} tone={holdersTone} />
          <Stat
            label="Mint Authority"
            value={stats.mintAuthority ? "Active" : "Disabled"}
            tone={stats.mintAuthority ? (token.verified ? undefined : "warn") : "good"}
          />
          <Stat
            label="Freeze Authority"
            value={stats.freezeAuthority ? "Active" : "Disabled"}
            tone={stats.freezeAuthority ? (token.verified ? undefined : "bad") : "good"}
          />
        </div>
        {stats.extensions.length > 0 && (
          <div className="mt-3 flex flex-wrap gap-1.5">
            {stats.extensions.map((extension) => (
              <span key={extension} className="rounded-md border border-line bg-surface-2 px-2 py-0.5 font-mono text-[11px] text-mute">
                {extension}
              </span>
            ))}
          </div>
        )}
      </Section>
      <Split>
        <Section title="AI Analysis" icon={<SparkIcon />}>
          <Analysis explanation={report.explanation} />
        </Section>
        <Section title="Safety Checks" icon={<ShieldIcon />} aside={`${report.risks.length} Checks`}>
          <RiskList risks={report.risks} />
        </Section>
      </Split>
      <div className="flex flex-wrap items-center gap-2 border-t border-line bg-surface-2 px-4 py-4 sm:px-5">
        <button type="button" onClick={() => onAsk(`Buy $5 of ${token.mint}`)} className={cx(buttonClass("primary"), "flex-1 sm:flex-none")}>
          <SwapIcon className="h-4 w-4" />
          Preview A $5 Buy
        </button>
        <a
          href={explorerToken(token.mint)}
          target="_blank"
          rel="noreferrer noopener"
          className={cx(buttonClass("secondary"), "flex-1 sm:flex-none")}
        >
          Solscan
          <ExternalIcon className="h-4 w-4" />
        </a>
      </div>
    </Card>
  );
}
