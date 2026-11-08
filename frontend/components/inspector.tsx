"use client";

import { useState } from "react";
import { postJson, streamEvents } from "@/lib/api";
import { cx, explorerAccount, shortAddress } from "@/lib/format";
import { applyEvent, createRun, failRun, finishRun, type Run } from "@/lib/runs";
import type { InspectionResult, SampleResponse } from "@/lib/types";
import {
  Address,
  Analysis,
  BalanceChanges,
  Card,
  ErrorBox,
  Notice,
  RiskList,
  Section,
  Split,
  VERDICT,
  VerdictHeader,
  buttonClass,
} from "./ui";
import { ThinkingTrace } from "./thinking";
import { AlertIcon, BoltIcon, CodeIcon, MessageIcon, SearchIcon, ShieldIcon, SparkIcon } from "./icons";

const TIPS = [
  { title: "Unlimited Approvals", text: "Lets someone move all of a token, forever." },
  { title: "Ownership Changes", text: "Hands a token account to another wallet." },
  { title: "Drain Transfers", text: "Moves most of your SOL to a stranger." },
];

function InspectionCard({ result }: { result: InspectionResult }) {
  return (
    <Card className={cx("animate-enter overflow-hidden", VERDICT[result.explanation.verdict].line)}>
      <VerdictHeader explanation={result.explanation} />
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 px-4 py-3 text-xs text-faint sm:px-5">
        <span className="inline-flex items-center gap-1">
          Viewed As <Address value={result.perspective} href={explorerAccount(result.perspective)} />
        </span>
        <span>{result.instructionCount} instructions</span>
        <span>{result.perspectiveSigns ? "Your wallet signs this" : "Your wallet does not sign this"}</span>
      </div>
      <Section title="What Changes In Your Wallet" icon={<BoltIcon />}>
        {result.simulation.ok ? (
          <BalanceChanges changes={result.simulation.changes} />
        ) : (
          <ErrorBox message={result.simulation.error ?? "The simulation failed."} />
        )}
      </Section>
      <Split>
        <Section title="AI Analysis" icon={<SparkIcon />}>
          <Analysis explanation={result.explanation} />
        </Section>
        <Section title="Safety Checks" icon={<ShieldIcon />} aside={`${result.risks.length} Checks`}>
          <RiskList risks={result.risks} />
        </Section>
      </Split>
      <Section title="Programs Called" icon={<CodeIcon />}>
        <div className="flex flex-wrap gap-1.5">
          {result.simulation.programs.map((program) => (
            <span
              key={program.id}
              title={program.id}
              className={cx(
                "inline-flex items-center gap-1.5 rounded-lg border px-2 py-1 text-xs",
                program.name ? "border-line bg-surface-2 text-ink" : "border-warn-line bg-warn-soft font-medium text-warn",
                !program.topLevel && "opacity-70",
              )}
            >
              {!program.name && <AlertIcon className="h-3 w-3" />}
              {program.name ?? shortAddress(program.id)}
            </span>
          ))}
        </div>
      </Section>
      {result.memos.length > 0 && (
        <Section title="Messages Inside The Transaction" icon={<MessageIcon />}>
          <div className="space-y-2">
            {result.memos.map((memo, index) => (
              <blockquote
                key={index}
                className="rounded-r-xl border-l-2 border-warn-solid bg-warn-soft px-3.5 py-2 font-mono text-[13px] text-ink"
              >
                {memo}
              </blockquote>
            ))}
            <p className="text-xs text-faint">Written by whoever built the transaction. Never trust urgent instructions here.</p>
          </div>
        </Section>
      )}
    </Card>
  );
}

export function Inspector({ owner }: { owner: string | null }) {
  const [text, setText] = useState("");
  const [run, setRun] = useState<Run | null>(null);
  const [busy, setBusy] = useState(false);
  const [sampleNote, setSampleNote] = useState<string | null>(null);
  const [sampleBusy, setSampleBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function inspect() {
    const transaction = text.trim();
    if (!transaction || busy) return;
    setBusy(true);
    setError(null);
    setRun(createRun("inspect"));
    try {
      await streamEvents("/api/inspect", { transaction, owner }, (event) =>
        setRun((current) => (current ? applyEvent(current, event) : current)),
      );
      setRun((current) => (current && !current.done ? finishRun(current) : current));
    } catch (err) {
      setRun((current) => (current ? failRun(current, err instanceof Error ? err.message : String(err)) : current));
    } finally {
      setBusy(false);
    }
  }

  async function loadSample() {
    if (!owner) {
      setError("Connect a wallet or use the demo wallet in the sidebar first. The sample targets that wallet.");
      return;
    }
    setSampleBusy(true);
    setError(null);
    try {
      const sample = await postJson<SampleResponse>("/api/sample", { owner });
      setText(sample.transaction);
      setSampleNote(sample.description);
      setRun(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSampleBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <div className="animate-enter px-1 pt-6 sm:pt-10 lg:pt-6">
        <div className="inline-flex items-center gap-2 rounded-full border border-line bg-surface px-3 py-1 text-xs font-medium text-mute shadow-sm">
          <ShieldIcon className="h-3.5 w-3.5 text-primary" />
          Transaction Inspector
        </div>
        <h1 className="mt-5 max-w-3xl text-[2.15rem] font-semibold leading-[1.05] tracking-tight text-ink sm:text-5xl 2xl:text-[3.5rem]">
          See Through Any <span className="text-primary">Transaction</span>.
        </h1>
        <p className="mt-4 max-w-2xl text-base leading-relaxed text-mute sm:text-lg">
          Paste a transaction a dApp wants you to sign. Preflight simulates it against your wallet and tells you exactly what
          it would do, including the tricks drainers use.
        </p>
        <div className="mt-6 grid max-w-3xl gap-2 sm:grid-cols-3">
          {TIPS.map((tip, index) => (
            <div
              key={tip.title}
              style={{ animationDelay: `${100 + index * 70}ms` }}
              className="animate-enter rounded-xl border border-line bg-surface p-3 shadow-(--shadow-card)"
            >
              <div className="flex items-center gap-2 text-sm font-semibold text-ink">
                <AlertIcon className="h-4 w-4 text-warn" />
                {tip.title}
              </div>
              <div className="mt-1 text-[13px] leading-relaxed text-mute">{tip.text}</div>
            </div>
          ))}
        </div>
      </div>
      <Card className="animate-enter p-4">
        <textarea
          value={text}
          onChange={(event) => {
            setText(event.target.value);
            setSampleNote(null);
          }}
          placeholder="Paste a base64-encoded transaction…"
          spellCheck={false}
          aria-label="Base64 transaction to inspect"
          className="scroll-thin h-32 w-full resize-none rounded-xl border border-line bg-surface-2 p-3 font-mono text-base leading-relaxed text-ink outline-none transition placeholder:text-faint focus:border-primary/50 focus:ring-4 focus:ring-primary/12 sm:h-36 sm:text-xs"
        />
        {sampleNote && (
          <div className="mt-3">
            <Notice>Sample loaded: {sampleNote}</Notice>
          </div>
        )}
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <button type="button" onClick={loadSample} disabled={sampleBusy} className={cx(buttonClass("danger"), "w-full sm:w-auto")}>
            {sampleBusy ? <span className="spinner" /> : <AlertIcon className="h-4 w-4" />}
            Load A Sample Wallet Drainer
          </button>
          <span className="order-last basis-full text-xs text-faint md:order-0 md:basis-auto">
            Built for your wallet, simulated only. Nothing is sent.
          </span>
          <button
            type="button"
            onClick={inspect}
            disabled={busy || !text.trim()}
            className={cx(buttonClass("primary"), "w-full sm:ml-auto sm:w-auto")}
          >
            {busy ? <span className="spinner" /> : <SearchIcon className="h-4 w-4" />}
            Inspect
          </button>
        </div>
      </Card>
      {error && <ErrorBox message={error} />}
      {run && (
        <div className="space-y-3">
          <ThinkingTrace run={run} />
          {run.notices.map((notice, index) => (
            <Notice key={index}>{notice}</Notice>
          ))}
          {run.inspection && <InspectionCard result={run.inspection} />}
          {run.error && <ErrorBox message={run.error} onRetry={inspect} />}
        </div>
      )}
    </div>
  );
}
