"use client";

import { useEffect, useState, type AnimationEvent, type ReactNode } from "react";
import type { BalanceChange, Explanation, RiskFlag, RiskLevel, Verdict } from "@/lib/types";
import type { Stage } from "@/lib/runs";
import { cx, formatAmount, formatSigned, formatSignedUsd, shortAddress, titleCase } from "@/lib/format";
import { useToast } from "./toast";
import {
  AlertIcon,
  CheckIcon,
  ChevronIcon,
  CopyIcon,
  ExternalIcon,
  InfoIcon,
  ShieldCheckIcon,
  SparkIcon,
  TrendDownIcon,
  TrendUpIcon,
  XCircleIcon,
} from "./icons";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

export function buttonClass(variant: ButtonVariant = "secondary", size: "sm" | "md" | "lg" = "md") {
  return cx(
    "inline-flex select-none items-center justify-center gap-2 rounded-xl font-semibold transition duration-200 active:scale-[0.98] disabled:pointer-events-none disabled:opacity-45",
    size === "sm" && "h-8 px-3 text-xs",
    size === "md" && "h-10 px-4 text-sm",
    size === "lg" && "h-11 px-5 text-sm",
    variant === "primary" && "bg-primary text-on-primary shadow-sm hover:bg-primary-hover",
    variant === "secondary" && "border border-line bg-surface text-ink hover:border-line-strong hover:bg-surface-2",
    variant === "ghost" && "text-mute hover:bg-surface-2 hover:text-ink",
    variant === "danger" && "border border-bad-line bg-bad-soft text-bad hover:border-bad",
  );
}

export const VERDICT: Record<Verdict, { label: string; text: string; soft: string; line: string; solid: string; step: number }> = {
  safe: { label: "Looks Safe", text: "text-ok", soft: "bg-ok-soft", line: "border-ok-line", solid: "bg-ok-solid", step: 0 },
  caution: { label: "Caution", text: "text-warn", soft: "bg-warn-soft", line: "border-warn-line", solid: "bg-warn-solid", step: 1 },
  danger: { label: "High Risk", text: "text-bad", soft: "bg-bad-soft", line: "border-bad-line", solid: "bg-bad-solid", step: 2 },
};

const LEVEL: Record<RiskLevel, { text: string; box: string; icon: typeof CheckIcon }> = {
  danger: { text: "text-bad", box: "border-bad-line bg-bad-soft", icon: XCircleIcon },
  warn: { text: "text-warn", box: "border-warn-line bg-warn-soft", icon: AlertIcon },
  info: { text: "text-info", box: "border-info-line bg-info-soft", icon: InfoIcon },
  ok: { text: "text-ok", box: "border-ok-line bg-ok-soft", icon: CheckIcon },
};

export function Card({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cx("rounded-2xl border border-line bg-surface shadow-(--shadow-card)", className)}>{children}</div>
  );
}

export function Presence({
  show,
  children,
  className,
}: {
  show: boolean;
  children: ReactNode;
  className?: string;
}) {
  const [mounted, setMounted] = useState(show);
  const [leaving, setLeaving] = useState(false);

  useEffect(() => {
    if (show) {
      setMounted(true);
      setLeaving(false);
    } else {
      setLeaving(true);
    }
  }, [show]);

  if (!mounted) return null;
  return (
    <div
      className={cx(className, leaving ? "animate-exit" : "animate-enter")}
      onAnimationEnd={(event: AnimationEvent<HTMLDivElement>) => {
        if (leaving && event.target === event.currentTarget) {
          setMounted(false);
          setLeaving(false);
        }
      }}
    >
      {children}
    </div>
  );
}

export function Section({
  title,
  icon,
  aside,
  children,
}: {
  title: string;
  icon?: ReactNode;
  aside?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="border-t border-line px-4 py-4 sm:px-5">
      <div className="mb-3 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.14em] text-mute">
        {icon && <span className="text-faint [&>svg]:h-3.5 [&>svg]:w-3.5">{icon}</span>}
        <span>{title}</span>
        {aside && <span className="ml-auto text-xs font-normal normal-case tracking-normal text-faint">{aside}</span>}
      </div>
      {children}
    </section>
  );
}

export function Split({ children }: { children: ReactNode }) {
  return (
    <div className="grid xl:grid-cols-2 xl:[&>section+section]:border-l xl:[&>section+section]:border-l-line">{children}</div>
  );
}

export function VerdictIcon({ verdict, className }: { verdict: Verdict; className?: string }) {
  if (verdict === "safe") return <ShieldCheckIcon className={className} />;
  if (verdict === "caution") return <AlertIcon className={className} />;
  return <XCircleIcon className={className} />;
}

export function ModelTag({ model }: { model: string }) {
  return (
    <span className="inline-flex items-center gap-1 rounded-md border border-line bg-surface px-1.5 py-0.5 font-mono text-[10px] text-mute">
      <SparkIcon className="h-3 w-3 text-primary-ink" />
      {model === "rules" ? "Rule Engine" : "AI Analysis"}
    </span>
  );
}

export function RiskMeter({ verdict }: { verdict: Verdict }) {
  const active = VERDICT[verdict].step;
  const segments = [
    { label: "Safe", fill: "bg-ok-solid" },
    { label: "Caution", fill: "bg-warn-solid" },
    { label: "Risk", fill: "bg-bad-solid" },
  ];
  return (
    <div className="flex items-center gap-1" aria-label={`Risk level: ${VERDICT[verdict].label}`}>
      {segments.map((segment, index) => (
        <span
          key={segment.label}
          className={cx(
            "h-1.5 rounded-full transition-all duration-500",
            index === active ? cx("w-7", segment.fill) : "w-3.5 bg-line-strong/60",
          )}
        />
      ))}
    </div>
  );
}

export function VerdictHeader({ explanation, aside }: { explanation: Explanation; aside?: ReactNode }) {
  const style = VERDICT[explanation.verdict];
  return (
    <div className={cx("rounded-t-2xl border-b px-4 pb-4 pt-4 sm:px-5 sm:pt-5", style.soft, style.line)}>
      <div className="flex items-start gap-3 sm:gap-3.5">
        <div className={cx("grid h-10 w-10 shrink-0 animate-pop place-items-center rounded-xl border bg-surface sm:h-11 sm:w-11", style.line, style.text)}>
          <VerdictIcon verdict={explanation.verdict} className="h-5 w-5" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className={cx("text-xs font-semibold uppercase tracking-[0.12em]", style.text)}>{style.label}</span>
            <RiskMeter verdict={explanation.verdict} />
            <ModelTag model={explanation.model} />
            {aside}
          </div>
          <h3 className="mt-1 text-base font-semibold leading-snug tracking-tight text-ink sm:text-lg">{titleCase(explanation.headline)}</h3>
        </div>
      </div>
    </div>
  );
}

export function Analysis({ explanation }: { explanation: Explanation }) {
  return (
    <div className="space-y-3">
      <p className="text-[15px] leading-relaxed text-ink">{explanation.summary}</p>
      {explanation.points.length > 0 && (
        <ul className="space-y-2">
          {explanation.points.map((point, index) => (
            <li
              key={index}
              style={{ animationDelay: `${index * 60}ms` }}
              className="flex animate-enter gap-2.5 text-sm leading-relaxed text-mute"
            >
              <span className="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-primary" />
              {point}
            </li>
          ))}
        </ul>
      )}
      <div className="flex gap-2.5 rounded-xl border border-primary/25 bg-primary-soft px-3.5 py-3 text-sm">
        <SparkIcon className="mt-0.5 h-4 w-4 shrink-0 text-primary-ink" />
        <p>
          <span className="font-semibold text-ink">Recommendation. </span>
          <span className="text-mute">{explanation.recommendation}</span>
        </p>
      </div>
    </div>
  );
}

export function TokenAvatar({ icon, symbol, size = 32 }: { icon: string | null; symbol: string; size?: number }) {
  const [failed, setFailed] = useState(false);
  if (icon && !failed) {
    return (
      <img
        src={icon}
        alt=""
        width={size}
        height={size}
        loading="lazy"
        referrerPolicy="no-referrer"
        style={{ width: size, height: size }}
        onError={() => setFailed(true)}
        className="shrink-0 rounded-full bg-surface-3 object-cover ring-1 ring-line"
      />
    );
  }
  return (
    <span
      style={{ width: size, height: size, fontSize: Math.max(10, size * 0.36) }}
      className="grid shrink-0 place-items-center rounded-full bg-surface-3 font-semibold text-mute ring-1 ring-line"
    >
      {symbol.slice(0, 2).toUpperCase()}
    </span>
  );
}

export function VerifiedBadge() {
  return (
    <span title="Verified Token" className="grid h-4 w-4 place-items-center rounded-full bg-ok-soft text-ok ring-1 ring-ok-line">
      <CheckIcon className="h-2.5 w-2.5" strokeWidth={3} />
    </span>
  );
}

export function BalanceChanges({ changes }: { changes: BalanceChange[] }) {
  if (changes.length === 0) {
    return <p className="text-sm text-mute">No balance changes for this wallet.</p>;
  }
  return (
    <div className="overflow-hidden rounded-xl border border-line">
      {changes.map((change, index) => {
        const up = change.delta >= 0;
        return (
          <div
            key={`${change.mint}-${change.native}`}
            style={{ animationDelay: `${index * 70}ms` }}
            className="flex animate-enter items-center gap-3 border-b border-line bg-surface px-3.5 py-3 last:border-b-0"
          >
            <TokenAvatar icon={change.icon} symbol={change.symbol} size={32} />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-1.5 text-sm font-medium text-ink">
                {change.symbol}
                {change.verified && <VerifiedBadge />}
              </div>
              <div className="truncate font-mono text-xs tabular-nums text-faint">
                {formatAmount(change.before)} → {formatAmount(change.after)}
              </div>
            </div>
            <div className="text-right">
              <div
                className={cx(
                  "inline-flex items-center gap-1 rounded-lg px-2 py-0.5 font-mono text-sm font-semibold tabular-nums",
                  up ? "bg-ok-soft text-ok" : "bg-bad-soft text-bad",
                )}
              >
                {up ? <TrendUpIcon className="h-3.5 w-3.5" /> : <TrendDownIcon className="h-3.5 w-3.5" />}
                {formatSigned(change.delta)}
              </div>
              <div className="mt-0.5 font-mono text-xs tabular-nums text-faint">{formatSignedUsd(change.usdDelta)}</div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function RiskRow({ risk, index }: { risk: RiskFlag; index: number }) {
  const style = LEVEL[risk.level];
  const LevelIcon = style.icon;
  return (
    <div
      style={{ animationDelay: `${index * 50}ms` }}
      className={cx("flex animate-enter gap-3 rounded-xl border px-3.5 py-2.5", style.box)}
    >
      <LevelIcon className={cx("mt-0.5 h-4 w-4 shrink-0", style.text)} />
      <div className="min-w-0">
        <div className={cx("text-sm font-semibold", style.text)}>{titleCase(risk.title)}</div>
        {risk.detail && <div className="mt-0.5 text-[13px] leading-relaxed text-mute">{risk.detail}</div>}
      </div>
    </div>
  );
}

export function RiskList({ risks }: { risks: RiskFlag[] }) {
  const [open, setOpen] = useState(false);
  const issues = risks.filter((risk) => risk.level !== "ok");
  const passed = risks.filter((risk) => risk.level === "ok");
  return (
    <div className="space-y-2">
      {issues.map((risk, index) => (
        <RiskRow key={`${risk.code}-${risk.title}`} risk={risk} index={index} />
      ))}
      {issues.length === 0 && (
        <p className="flex items-center gap-2 text-sm font-medium text-ok">
          <ShieldCheckIcon className="h-4 w-4" />
          No issues found.
        </p>
      )}
      {passed.length > 0 && (
        <div className="pt-1">
          <button
            type="button"
            onClick={() => setOpen((value) => !value)}
            aria-expanded={open}
            className="flex items-center gap-2 rounded-lg py-1 text-sm text-mute transition hover:text-ink"
          >
            <span className="grid h-5 w-5 place-items-center rounded-full bg-ok-soft text-ok ring-1 ring-ok-line">
              <CheckIcon className="h-3 w-3" strokeWidth={3} />
            </span>
            {passed.length} {passed.length === 1 ? "Check" : "Checks"} Passed
            <ChevronIcon className={cx("h-4 w-4 transition duration-300", open && "rotate-180")} />
          </button>
          <Presence show={open} className="mt-2 space-y-2">
            {passed.map((risk, index) => (
              <RiskRow key={`${risk.code}-${risk.title}`} risk={risk} index={index} />
            ))}
          </Presence>
        </div>
      )}
    </div>
  );
}

function StageDot({ status }: { status: Stage["status"] }) {
  if (status === "active") return <span className="spinner text-primary" />;
  if (status === "done") return <CheckIcon className="h-3.5 w-3.5 animate-pop text-ok" strokeWidth={3} />;
  if (status === "error") return <XCircleIcon className="h-3.5 w-3.5 animate-pop text-bad" />;
  return <span className="h-1.5 w-1.5 rounded-full bg-line-strong" />;
}

export function StageTrack({ stages }: { stages: Stage[] }) {
  if (stages.length === 0) return null;
  const active = stages.find((stage) => stage.status === "active");
  const latest = [...stages].reverse().find((stage) => stage.note && stage.status !== "idle");
  return (
    <div className="space-y-1.5">
      <ol className="flex flex-wrap items-center gap-1.5">
        {stages.map((stage, index) => (
          <li key={stage.id} className="flex animate-fade items-center gap-1.5">
            <span
              title={stage.note}
              className={cx(
                "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium transition duration-300",
                stage.status === "active" && "border-primary/40 bg-primary-soft text-primary-ink",
                stage.status === "done" && "border-line bg-surface text-ink",
                stage.status === "error" && "border-bad-line bg-bad-soft text-bad",
                (stage.status === "idle" || stage.status === "skipped") && "border-line text-faint",
                stage.status === "skipped" && "line-through decoration-line-strong",
              )}
            >
              <StageDot status={stage.status} />
              {stage.label}
            </span>
            {index < stages.length - 1 && (
              <span className={cx("h-px w-3 transition-colors duration-500", stage.status === "done" ? "bg-ok-solid" : "bg-line-strong")} />
            )}
          </li>
        ))}
      </ol>
      {(active || latest) && (
        <p key={active?.id ?? latest?.id} className="animate-fade truncate pl-1 text-xs text-faint">
          {active ? `${active.label}…` : latest?.note}
        </p>
      )}
    </div>
  );
}

export function CopyButton({ value, label }: { value: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  const toast = useToast();
  return (
    <button
      type="button"
      title="Copy"
      onClick={() => {
        void navigator.clipboard.writeText(value).then(() => {
          setCopied(true);
          toast({ title: "Copied to clipboard", tone: "ok" });
          window.setTimeout(() => setCopied(false), 1400);
        });
      }}
      className="inline-flex items-center gap-1 rounded-md px-1.5 py-1 text-xs text-mute transition hover:bg-surface-2 hover:text-ink"
    >
      {copied ? <CheckIcon className="h-3.5 w-3.5 animate-pop text-ok" /> : <CopyIcon className="h-3.5 w-3.5" />}
      {label}
    </button>
  );
}

export function Address({ value, href }: { value: string; href?: string }) {
  return (
    <span className="inline-flex items-center gap-0.5">
      <code className="font-mono text-xs text-ink">{shortAddress(value, 6)}</code>
      <CopyButton value={value} />
      {href && (
        <a
          href={href}
          target="_blank"
          rel="noreferrer noopener"
          title="Open in explorer"
          className="rounded-md p-1 text-mute transition hover:bg-surface-2 hover:text-ink"
        >
          <ExternalIcon className="h-3.5 w-3.5" />
        </a>
      )}
    </span>
  );
}

export function Notice({ children }: { children: ReactNode }) {
  return (
    <div className="flex animate-enter gap-2 rounded-xl border border-info-line bg-info-soft px-3 py-2 text-[13px] text-ink">
      <InfoIcon className="mt-0.5 h-4 w-4 shrink-0 text-info" />
      <span>{children}</span>
    </div>
  );
}

export function ErrorBox({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="flex animate-enter items-start gap-3 rounded-2xl border border-bad-line bg-bad-soft px-4 py-3">
      <XCircleIcon className="mt-0.5 h-5 w-5 shrink-0 text-bad" />
      <p className="flex-1 text-sm leading-relaxed text-ink">{message}</p>
      {onRetry && (
        <button type="button" onClick={onRetry} className={buttonClass("secondary", "sm")}>
          Try Again
        </button>
      )}
    </div>
  );
}

export function Stat({ label, value, tone }: { label: string; value: ReactNode; tone?: "good" | "bad" | "warn" }) {
  return (
    <div className="rounded-xl border border-line bg-surface-2 px-3 py-2.5">
      <div className="text-[11px] uppercase tracking-widest text-faint">{label}</div>
      <div
        className={cx(
          "mt-1 truncate font-mono text-sm font-semibold tabular-nums",
          tone === "good" && "text-ok",
          tone === "bad" && "text-bad",
          tone === "warn" && "text-warn",
          !tone && "text-ink",
        )}
      >
        {value}
      </div>
    </div>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return (
    <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded-md border border-line bg-surface-2 px-1 font-mono text-[10px] font-medium text-mute">
      {children}
    </kbd>
  );
}

export function Skeleton({ className }: { className?: string }) {
  return <span className={cx("skeleton block rounded-md", className)} />;
}
