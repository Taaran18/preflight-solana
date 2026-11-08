import type {
  InspectionResult,
  Intent,
  PlanResult,
  Portfolio,
  StageDef,
  StageStatus,
  StreamEvent,
  TokenReport,
} from "./types";

export interface Stage extends StageDef {
  status: StageStatus;
  note?: string;
}

export interface Run {
  id: string;
  prompt: string;
  stages: Stage[];
  intent?: Intent;
  parsedBy?: string;
  message?: string;
  notices: string[];
  plan?: PlanResult;
  report?: TokenReport;
  inspection?: InspectionResult;
  portfolio?: Portfolio;
  error?: string;
  done: boolean;
  startedAt: number;
  finishedAt?: number;
}

let counter = 0;

export function createRun(prompt: string): Run {
  counter += 1;
  return { id: `${Date.now()}-${counter}`, prompt, stages: [], notices: [], done: false, startedAt: Date.now() };
}

export function finishRun(run: Run): Run {
  return {
    ...run,
    done: true,
    finishedAt: run.finishedAt ?? Date.now(),
    stages: run.stages.map((stage) => {
      if (stage.status === "idle") return { ...stage, status: "skipped" };
      if (stage.status === "active") return { ...stage, status: run.error ? "error" : "done" };
      return stage;
    }),
  };
}

export function failRun(run: Run, message: string): Run {
  return finishRun({
    ...run,
    error: message,
    stages: run.stages.map((stage) => (stage.status === "active" ? { ...stage, status: "error" } : stage)),
  });
}

export function applyEvent(run: Run, event: StreamEvent): Run {
  switch (event.type) {
    case "stages": {
      const previous = new Map(run.stages.map((stage) => [stage.id, stage]));
      return {
        ...run,
        stages: event.stages.map((stage) => previous.get(stage.id) ?? { ...stage, status: "idle" }),
      };
    }
    case "stage":
      return {
        ...run,
        stages: run.stages.map((stage) =>
          stage.id === event.id ? { ...stage, status: event.status, note: event.note ?? stage.note } : stage,
        ),
      };
    case "intent":
      return { ...run, intent: event.intent, parsedBy: event.parsedBy };
    case "message":
      return { ...run, message: event.text };
    case "plan":
      return { ...run, plan: event.plan };
    case "token_report":
      return { ...run, report: event.report };
    case "inspection":
      return { ...run, inspection: event.result };
    case "portfolio":
      return { ...run, portfolio: event.portfolio };
    case "notice":
      return { ...run, notices: [...run.notices, event.text] };
    case "error":
      return failRun(run, event.message);
    case "done":
      return finishRun(run);
  }
}
