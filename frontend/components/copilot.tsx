"use client";

import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { streamEvents } from "@/lib/api";
import { cx, formatAmount, formatUsd, shortAddress } from "@/lib/format";
import { applyEvent, createRun, failRun, finishRun, type Run } from "@/lib/runs";
import type { Activity, Portfolio } from "@/lib/types";
import { PlanCard } from "./plan-card";
import { TokenCard } from "./token-card";
import { Card, ErrorBox, Kbd, Notice, TokenAvatar, buttonClass } from "./ui";
import { ThinkingTrace } from "./thinking";
import { useToast } from "./toast";
import {
  BoltIcon,
  CheckIcon,
  ClockIcon,
  CoinIcon,
  LayersIcon,
  Logo,
  SendIcon,
  ShieldCheckIcon,
  ShieldIcon,
  SparkIcon,
  SwapIcon,
  WalletIcon,
  PlusIcon,
} from "./icons";

const EXAMPLES = [
  { text: "Swap 0.05 SOL to USDC", icon: SwapIcon },
  { text: "Buy $5 of JUP", icon: CoinIcon },
  { text: "Swap half my USDC to SOL", icon: SwapIcon },
  { text: "Is BONK safe?", icon: ShieldIcon },
  { text: "What's in my wallet?", icon: WalletIcon },
];

const STATS = [
  { icon: ClockIcon, value: "≈1s", label: "To Confirm On Solana" },
  { icon: BoltIcon, value: "<$0.001", label: "Typical Network Fee" },
  { icon: LayersIcon, value: "20+", label: "Automatic Safety Checks" },
];

const PREVIEW_STEPS = ["Understand", "Route", "Simulate", "Analyze", "Guard"];

function HeroPreview() {
  const [step, setStep] = useState(0);

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setStep(PREVIEW_STEPS.length + 1);
      return;
    }
    const timer = window.setInterval(() => setStep((value) => (value > PREVIEW_STEPS.length + 2 ? 0 : value + 1)), 850);
    return () => window.clearInterval(timer);
  }, []);

  const finished = step > PREVIEW_STEPS.length;
  return (
    <div className="relative hidden xl:block">
      <div className="dots absolute -inset-6 rounded-[28px] opacity-80" />
      <Card className="relative overflow-hidden">
        <div className="flex items-center gap-2 border-b border-line px-4 py-3">
          <span className="rounded-md bg-surface-2 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-widest text-faint">
            Example
          </span>
          <span className="text-sm font-medium text-ink">Swap 0.5 SOL To USDC</span>
        </div>
        <div className="space-y-2 p-4">
          {PREVIEW_STEPS.map((label, index) => {
            const done = step > index;
            const active = step === index;
            return (
              <div key={label} className="flex items-center gap-3">
                <span
                  className={cx(
                    "grid h-6 w-6 place-items-center rounded-full border transition duration-300",
                    done && "border-ok-line bg-ok-soft text-ok",
                    active && "border-primary/40 bg-primary-soft text-primary",
                    !done && !active && "border-line text-faint",
                  )}
                >
                  {done ? (
                    <CheckIcon className="h-3.5 w-3.5 animate-pop" strokeWidth={3} />
                  ) : active ? (
                    <span className="spinner h-3 w-3" />
                  ) : (
                    <span className="h-1.5 w-1.5 rounded-full bg-line-strong" />
                  )}
                </span>
                <span className={cx("text-sm transition-colors duration-300", done || active ? "text-ink" : "text-faint")}>{label}</span>
                <span className="ml-auto h-1.5 w-16 overflow-hidden rounded-full bg-surface-3">
                  <span
                    className={cx("block h-full rounded-full bg-primary transition-all duration-700", done ? "w-full" : active ? "w-1/2" : "w-0")}
                  />
                </span>
              </div>
            );
          })}
        </div>
        <div className="grid grid-cols-2 gap-2 border-t border-line p-4">
          <div className="rounded-xl bg-surface-2 p-3">
            <div className="text-[10px] uppercase tracking-widest text-faint">You Pay</div>
            <div className="mt-1 font-mono text-sm font-semibold tabular-nums text-ink">0.5 SOL</div>
          </div>
          <div className="rounded-xl bg-surface-2 p-3">
            <div className="text-[10px] uppercase tracking-widest text-faint">You Receive</div>
            <div className="mt-1 font-mono text-sm font-semibold tabular-nums text-ink">≈ 44.8 USDC</div>
          </div>
        </div>
        <div className="flex h-12 items-center border-t border-line px-4">
          {finished ? (
            <span key="done" className="inline-flex animate-pop items-center gap-2 rounded-lg border border-ok-line bg-ok-soft px-2.5 py-1 text-xs font-semibold text-ok">
              <ShieldCheckIcon className="h-3.5 w-3.5" />
              Looks Safe · Guarded On-Chain
            </span>
          ) : (
            <span key="pending" className="animate-fade text-xs text-faint">
              Checking Before You Sign…
            </span>
          )}
        </div>
      </Card>
    </div>
  );
}

function Hero({ onPick, onDemo }: { onPick: (text: string) => void; onDemo: (() => void) | null }) {
  return (
    <div className="grid items-center gap-8 px-1 pb-2 pt-2 sm:pt-6 lg:pt-3 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)] 2xl:gap-12">
      <div className="animate-enter">
        <h1 className="max-w-4xl text-[2.15rem] font-semibold leading-[1.05] tracking-tight text-ink sm:text-5xl 2xl:text-[3.5rem]">
          Every Transaction, <span className="text-primary">Explained</span> Before You Sign.
        </h1>
        <p className="mt-4 max-w-3xl text-base leading-relaxed text-mute sm:text-[17px]">
          Tell Preflight what you want. It finds the best route, simulates the transaction, explains every effect in plain
          language, and locks the result on-chain before your wallet ever asks you to sign.
        </p>
        {onDemo && (
          <div className="mt-5 flex max-w-2xl flex-col gap-3 rounded-2xl border border-primary/25 bg-primary-soft p-3.5 sm:flex-row sm:items-center">
            <div className="flex-1">
              <div className="text-sm font-semibold text-ink">No Wallet Yet?</div>
              <div className="mt-0.5 text-[13px] leading-relaxed text-mute">
                Try everything with a demo wallet: real mainnet balances and simulations, view only.
              </div>
            </div>
            <button type="button" onClick={onDemo} className={buttonClass("primary")}>
              <WalletIcon className="h-4 w-4" />
              Use The Demo Wallet
            </button>
          </div>
        )}
        <div className="mt-5 grid max-w-2xl grid-cols-3 gap-2 sm:gap-3">
          {STATS.map(({ icon: Icon, value, label }, index) => (
            <div
              key={label}
              style={{ animationDelay: `${120 + index * 80}ms` }}
              className="animate-enter rounded-2xl border border-line bg-surface p-3 shadow-(--shadow-card) sm:p-4"
            >
              <Icon className="h-4 w-4 text-primary" />
              <div className="mt-2 font-mono text-lg font-semibold tabular-nums text-ink sm:text-xl">{value}</div>
              <div className="mt-0.5 text-[11px] leading-snug text-mute sm:text-xs">{label}</div>
            </div>
          ))}
        </div>
        <div className="mt-6 text-xs font-semibold uppercase tracking-[0.14em] text-faint">Try One</div>
        <div className="mt-2.5 flex flex-wrap gap-2">
          {EXAMPLES.map(({ text, icon: Icon }, index) => (
            <button
              key={text}
              type="button"
              onClick={() => onPick(text)}
              style={{ animationDelay: `${200 + index * 50}ms` }}
              className="group inline-flex animate-enter items-center gap-2 rounded-xl border border-line bg-surface px-3.5 py-2 text-sm text-ink shadow-sm transition hover:-translate-y-0.5 hover:border-primary/40 hover:bg-primary-soft active:translate-y-0"
            >
              <Icon className="h-4 w-4 text-faint transition group-hover:text-primary" />
              {text}
            </button>
          ))}
        </div>
      </div>
      <HeroPreview />
    </div>
  );
}

function PortfolioReply({ portfolio }: { portfolio: Portfolio }) {
  return (
    <Card className="animate-enter p-4">
      <div className="flex items-baseline justify-between">
        <span className="text-xs font-semibold uppercase tracking-[0.12em] text-faint">Wallet Value</span>
        <span className="font-mono text-lg font-semibold tabular-nums">{formatUsd(portfolio.totalUsd)}</span>
      </div>
      <div className="mt-3 space-y-2">
        {portfolio.holdings.slice(0, 6).map((holding, index) => (
          <div
            key={`${holding.mint}-${holding.native}`}
            style={{ animationDelay: `${index * 50}ms` }}
            className="flex animate-enter items-center gap-3"
          >
            <TokenAvatar icon={holding.icon} symbol={holding.symbol} size={26} />
            <span className="text-sm font-medium">{holding.symbol}</span>
            <span className="ml-auto font-mono text-sm tabular-nums text-ink">{formatAmount(holding.amount)}</span>
            <span className="w-20 text-right font-mono text-xs tabular-nums text-faint">{formatUsd(holding.usdValue)}</span>
          </div>
        ))}
      </div>
    </Card>
  );
}

function RunView({
  run,
  onRetry,
  onAsk,
  onExecuted,
}: {
  run: Run;
  onRetry: () => void;
  onAsk: (text: string) => void;
  onExecuted: (activity: Activity) => void;
}) {
  const reply = run.intent?.reply;
  return (
    <div className="animate-enter space-y-4">
      <div className="flex justify-end">
        <div className="max-w-[88%] wrap-break-word rounded-2xl rounded-br-md bg-primary px-4 py-2.5 text-[15px] leading-relaxed text-on-primary shadow-sm md:max-w-[75%] xl:max-w-[60%]">
          {run.prompt}
        </div>
      </div>
      <div className="flex gap-0 sm:gap-3">
        <Logo className="mt-0.5 hidden h-7 w-7 shrink-0 sm:block" />
        <div className="min-w-0 flex-1 space-y-3">
          <ThinkingTrace run={run} />
          {reply && (
            <p className="animate-fade text-[15px] leading-relaxed text-ink">
              {reply}
            </p>
          )}
          {run.message && run.message !== reply && <p className="animate-fade text-[15px] leading-relaxed text-ink">{run.message}</p>}
          {run.notices.map((notice, index) => (
            <Notice key={index}>{notice}</Notice>
          ))}
          {run.portfolio && <PortfolioReply portfolio={run.portfolio} />}
          {run.plan && <PlanCard plan={run.plan} onExecuted={onExecuted} onRefresh={onRetry} />}
          {run.report && <TokenCard report={run.report} onAsk={onAsk} />}
          {run.error && <ErrorBox message={run.error} onRetry={onRetry} />}
        </div>
      </div>
    </div>
  );
}

export function Copilot({
  owner,
  connected,
  draft,
  onExecuted,
  onAsk,
  onDemo,
}: {
  owner: string | null;
  connected: boolean;
  portfolio: Portfolio | null;
  draft: { text: string; nonce: number } | null;
  onExecuted: (activity: Activity) => void;
  onAsk: (text: string) => void;
  onDemo: (() => void) | null;
}) {
  const [runs, setRuns] = useState<Run[]>([]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const queuedRef = useRef<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const endRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const toast = useToast();

  function newChat() {
    abortRef.current?.abort();
    abortRef.current = null;
    queuedRef.current = null;
    setRuns([]);
    setInput("");
    toast({ title: "Started A Fresh Chat", tone: "info" });
    window.setTimeout(() => inputRef.current?.focus(), 50);
  }

  async function submit(text: string) {
    const prompt = text.trim().slice(0, 2000);
    if (!prompt) return;
    if (busyRef.current) {
      queuedRef.current = prompt;
      setInput("");
      return;
    }
    busyRef.current = true;
    setBusy(true);
    setInput("");
    const run = createRun(prompt);
    const update = (change: (current: Run) => Run) =>
      setRuns((current) => current.map((item) => (item.id === run.id ? change(item) : item)));
    setRuns((current) => [...current, run]);
    const controller = new AbortController();
    abortRef.current = controller;
    try {
      await streamEvents(
        "/api/plan",
        { prompt, owner },
        (event) => update((current) => applyEvent(current, event)),
        controller.signal,
      );
      update((current) => (current.done ? current : finishRun(current)));
    } catch (err) {
      if (!controller.signal.aborted) {
        update((current) => failRun(current, err instanceof Error ? err.message : String(err)));
      }
    } finally {
      if (abortRef.current === controller) abortRef.current = null;
      busyRef.current = false;
      setBusy(false);
      const queued = queuedRef.current;
      queuedRef.current = null;
      if (queued) void submit(queued);
    }
  }

  useEffect(() => {
    if (draft) void submit(draft.text);
  }, [draft]);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [runs.length]);

  useEffect(() => {
    function onKey(event: globalThis.KeyboardEvent) {
      const target = event.target as HTMLElement | null;
      const typing = target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);
      if (event.key === "/" && !typing && inputRef.current && inputRef.current.offsetParent !== null) {
        event.preventDefault();
        inputRef.current.focus();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  function onSubmit(event: FormEvent) {
    event.preventDefault();
    void submit(input);
  }

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      void submit(input);
    }
  }

  return (
    <div className="flex min-h-[calc(100dvh-9rem)] flex-col lg:h-full lg:min-h-0">
      {runs.length > 0 && (
        <div className="sticky top-14 z-20 -mx-4 flex shrink-0 animate-fade items-center gap-2 border-b border-line bg-bg/90 px-4 py-3 backdrop-blur-xl sm:top-16 sm:-mx-6 sm:px-6 lg:static lg:mx-0 lg:mb-1 lg:px-0 lg:pt-4">
          <span className="text-xs font-semibold uppercase tracking-[0.14em] text-faint">Conversation</span>
          <span className="rounded-md bg-surface-2 px-1.5 py-0.5 font-mono text-[10px] font-semibold tabular-nums text-mute">
            {runs.length}
          </span>
          <button type="button" onClick={newChat} className={cx(buttonClass("secondary", "sm"), "ml-auto")}>
            <PlusIcon className="h-3.5 w-3.5" />
            New Chat
          </button>
        </div>
      )}
      <div className="scroll-thin flex-1 space-y-8 pt-5 lg:min-h-0 lg:overflow-x-hidden lg:overflow-y-auto lg:pb-2 lg:pr-1 lg:pt-5">
        {runs.length === 0 ? (
          <Hero
            onDemo={onDemo}
            onPick={(text) => {
              setInput(text);
              inputRef.current?.focus();
            }}
          />
        ) : (
          runs.map((run) => (
            <RunView key={run.id} run={run} onRetry={() => void submit(run.prompt)} onAsk={onAsk} onExecuted={onExecuted} />
          ))
        )}
        <div ref={endRef} />
      </div>
      <form onSubmit={onSubmit} className="sticky bottom-[max(0.75rem,env(safe-area-inset-bottom))] z-10 mt-8 sm:bottom-4 lg:static lg:mt-3 lg:shrink-0 lg:pb-4">
        <div className="rounded-2xl border border-line bg-surface p-2 shadow-(--shadow-pop) transition focus-within:border-primary/50 focus-within:ring-4 focus-within:ring-primary/12">
          <textarea
            ref={inputRef}
            rows={1}
            value={input}
            maxLength={2000}
            onChange={(event) => setInput(event.target.value)}
            onKeyDown={onKeyDown}
            aria-label="Tell Preflight what you want to do"
            placeholder={owner ? "Swap half my SOL to USDC…" : "Is WIF safe?  ·  connect a wallet to trade"}
            className="field-sizing-content max-h-40 min-h-11 w-full resize-none bg-transparent px-3 py-2.5 text-base text-ink outline-none placeholder:text-faint sm:text-[15px]"
          />
          <div className="flex items-center gap-2 px-2 pb-1">
            <span className="flex min-w-0 items-center gap-1.5 truncate text-xs text-faint">
              <span className={cx("h-1.5 w-1.5 shrink-0 rounded-full", owner ? (connected ? "bg-ok-solid" : "bg-warn-solid") : "bg-line-strong")} />
              {owner
                ? connected
                  ? `Planning For ${shortAddress(owner)}`
                  : `Simulating As ${shortAddress(owner)} · View Only`
                : "No Wallet · Token Checks Only"}
            </span>
            <span className="ml-auto hidden items-center gap-1 text-[11px] text-faint md:flex">
              <Kbd>/</Kbd> Focus <Kbd>↵</Kbd> Send
            </span>
            <button
              type="submit"
              disabled={busy || !input.trim()}
              className={cx(buttonClass("primary"), "ml-auto shrink-0 md:ml-2")}
            >
              {busy ? <span className="spinner" /> : <SendIcon className="h-4 w-4" />}
              <span className="hidden min-[420px]:inline">{busy ? "Working" : "Preflight It"}</span>
            </button>
          </div>
        </div>
        <p className="mt-2 hidden text-center text-[11px] text-faint sm:block">
          <SparkIcon className="mr-1 inline h-3 w-3 align-[-2px]" />
          AI can be wrong. Every number shown comes from an on-chain simulation, and you always approve in your own wallet.
        </p>
      </form>
    </div>
  );
}
