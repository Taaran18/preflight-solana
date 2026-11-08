"use client";

import { useEffect, useState } from "react";
import { cx } from "@/lib/format";
import type { Run, Stage } from "@/lib/runs";
import { Presence } from "./ui";
import { CheckIcon, ChevronIcon, XCircleIcon } from "./icons";

const STEP_COPY: Record<string, string> = {
  connect: "Connecting",
  understand: "Reading Your Request",
  route: "Finding The Best Route",
  prepare: "Preparing The Transfer",
  simulate: "Simulating On Mainnet",
  analyze: "Checking Risks & Writing The Analysis",
  lookup: "Looking Up The Token",
  audit: "Auditing It On-Chain",
  decode: "Decoding The Transaction",
};

function stepLabel(stage: Stage) {
  return STEP_COPY[stage.id] ?? stage.label;
}

function StepDot({ status }: { status: Stage["status"] }) {
  if (status === "active") {
    return (
      <span className="grid h-6 w-6 place-items-center rounded-full border border-primary/40 bg-primary-soft text-primary">
        <span className="spinner h-3 w-3" />
      </span>
    );
  }
  if (status === "done") {
    return (
      <span className="grid h-6 w-6 animate-pop place-items-center rounded-full border border-ok-line bg-ok-soft text-ok">
        <CheckIcon className="h-3.5 w-3.5" strokeWidth={3} />
      </span>
    );
  }
  if (status === "error") {
    return (
      <span className="grid h-6 w-6 animate-pop place-items-center rounded-full border border-bad-line bg-bad-soft text-bad">
        <XCircleIcon className="h-3.5 w-3.5" />
      </span>
    );
  }
  return (
    <span className="grid h-6 w-6 place-items-center rounded-full border border-line bg-surface">
      <span className="h-1.5 w-1.5 rounded-full bg-line-strong" />
    </span>
  );
}

export function ThinkingTrace({ run }: { run: Run }) {
  const [open, setOpen] = useState(true);
  const [now, setNow] = useState(() => Date.now());
  const running = !run.done;

  useEffect(() => {
    if (!running) return;
    const timer = window.setInterval(() => setNow(Date.now()), 100);
    return () => window.clearInterval(timer);
  }, [running]);

  useEffect(() => {
    if (run.done && !run.error) {
      const timer = window.setTimeout(() => setOpen(false), 1100);
      return () => window.clearTimeout(timer);
    }
  }, [run.done, run.error]);

  const stages: Stage[] = run.stages.length > 0 ? run.stages : [{ id: "connect", label: "Connecting", status: "active" }];
  const active = stages.find((stage) => stage.status === "active");
  const failed = Boolean(run.error) || stages.some((stage) => stage.status === "error");
  const finished = stages.filter((stage) => stage.status === "done").length;
  const visible = stages.filter((stage) => stage.status !== "skipped");
  const elapsed = Math.max(0, ((run.finishedAt ?? now) - run.startedAt) / 1000);
  const reply = run.intent?.reply;

  const title = running ? `${active ? stepLabel(active) : "Thinking"}…` : failed ? "Stopped" : "Done";
  const subtitle = running ? `Step ${Math.min(finished + 1, stages.length)} Of ${stages.length}` : `${visible.length} Steps`;

  return (
    <div className="w-full max-w-xl overflow-hidden rounded-2xl border border-line bg-surface shadow-(--shadow-card)">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        className="flex w-full items-center gap-3 px-3.5 py-2.5 text-left transition hover:bg-surface-2"
      >
        <span
          className={cx(
            "grid h-8 w-8 shrink-0 place-items-center rounded-full transition-colors duration-300",
            running && "bg-primary-soft text-primary",
            !running && failed && "bg-bad-soft text-bad",
            !running && !failed && "bg-ok-soft text-ok",
          )}
        >
          {running ? (
            <span className="spinner" />
          ) : failed ? (
            <XCircleIcon className="h-4 w-4 animate-pop" />
          ) : (
            <CheckIcon className="h-4 w-4 animate-pop" strokeWidth={3} />
          )}
        </span>
        <span className="min-w-0 flex-1">
          <span key={title} className={cx("block animate-fade truncate text-sm font-semibold text-ink", running && "thinking-text")}>
            {title}
          </span>
          <span className="block text-xs text-faint">{subtitle}</span>
        </span>
        <span className="rounded-md bg-surface-2 px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-mute">{elapsed.toFixed(1)}s</span>
        <ChevronIcon className={cx("h-4 w-4 shrink-0 text-faint transition duration-300", open && "rotate-180")} />
      </button>
      {running && <div className="progress-track" />}
      <Presence show={open}>
        <ol className="border-t border-line px-3.5 py-3">
          {visible.map((stage, index) => {
            const note = stage.note && stage.note !== reply ? stage.note : null;
            return (
              <li
                key={stage.id}
                style={{ animationDelay: `${index * 40}ms` }}
                className="relative flex animate-enter gap-3 pb-3 last:pb-0"
              >
                {index < visible.length - 1 && (
                  <span
                    className={cx(
                      "absolute bottom-0 left-[11.5px] top-6 w-px transition-colors duration-500",
                      stage.status === "done" ? "bg-ok-line" : "bg-line",
                    )}
                  />
                )}
                <span className="relative z-10 shrink-0">
                  <StepDot status={stage.status} />
                </span>
                <div className="min-w-0 pt-0.5">
                  <div
                    className={cx(
                      "text-sm transition-colors duration-300",
                      stage.status === "active" && "font-semibold text-ink",
                      stage.status === "done" && "text-ink",
                      stage.status === "error" && "font-medium text-bad",
                      stage.status === "idle" && "text-faint",
                    )}
                  >
                    {stepLabel(stage)}
                  </div>
                  {note && <div className="mt-0.5 animate-fade text-xs leading-relaxed text-faint">{note}</div>}
                </div>
              </li>
            );
          })}
        </ol>
      </Presence>
    </div>
  );
}
