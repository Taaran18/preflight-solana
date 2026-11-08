"use client";

import { createContext, useCallback, useContext, useRef, useState, type ReactNode } from "react";
import { cx } from "@/lib/format";
import { CheckIcon, CloseIcon, InfoIcon, XCircleIcon } from "./icons";

type Tone = "ok" | "info" | "bad";

interface ToastInput {
  title: string;
  detail?: string;
  tone?: Tone;
}

interface ToastItem extends Required<Omit<ToastInput, "detail">> {
  id: number;
  detail?: string;
  leaving: boolean;
}

const ToastContext = createContext<(toast: ToastInput) => void>(() => {});

export function useToast() {
  return useContext(ToastContext);
}

const TONE: Record<Tone, { icon: typeof CheckIcon; box: string }> = {
  ok: { icon: CheckIcon, box: "bg-ok-soft text-ok" },
  info: { icon: InfoIcon, box: "bg-info-soft text-info" },
  bad: { icon: XCircleIcon, box: "bg-bad-soft text-bad" },
};

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const sequence = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((list) => list.map((toast) => (toast.id === id ? { ...toast, leaving: true } : toast)));
  }, []);

  const remove = useCallback((id: number) => {
    setToasts((list) => list.filter((toast) => toast.id !== id));
  }, []);

  const push = useCallback(
    ({ title, detail, tone = "info" }: ToastInput) => {
      sequence.current += 1;
      const id = sequence.current;
      setToasts((list) => [...list.slice(-2), { id, title, detail, tone, leaving: false }]);
      window.setTimeout(() => dismiss(id), 3600);
    },
    [dismiss],
  );

  return (
    <ToastContext.Provider value={push}>
      {children}
      <div
        aria-live="polite"
        className="pointer-events-none fixed inset-x-0 top-16 z-50 flex flex-col items-center gap-2 px-4 sm:inset-x-auto sm:right-6 sm:top-20 sm:items-end"
      >
        {toasts.map((toast) => {
          const tone = TONE[toast.tone];
          const ToneIcon = tone.icon;
          return (
            <div
              key={toast.id}
              role="status"
              onAnimationEnd={(event) => {
                if (toast.leaving && event.target === event.currentTarget) remove(toast.id);
              }}
              className={cx(
                "pointer-events-auto flex w-full max-w-sm items-start gap-3 rounded-2xl border border-line bg-surface px-3.5 py-3 shadow-(--shadow-pop)",
                toast.leaving ? "animate-toast-out" : "animate-toast-in",
              )}
            >
              <span className={cx("grid h-7 w-7 shrink-0 place-items-center rounded-full", tone.box)}>
                <ToneIcon className="h-4 w-4" />
              </span>
              <div className="min-w-0 flex-1 pt-0.5">
                <div className="text-sm font-medium text-ink">{toast.title}</div>
                {toast.detail && <div className="mt-0.5 text-[13px] leading-relaxed text-mute">{toast.detail}</div>}
              </div>
              <button
                type="button"
                aria-label="Dismiss"
                onClick={() => dismiss(toast.id)}
                className="rounded-md p-1 text-faint transition hover:bg-surface-2 hover:text-ink"
              >
                <CloseIcon className="h-3.5 w-3.5" />
              </button>
            </div>
          );
        })}
      </div>
    </ToastContext.Provider>
  );
}
