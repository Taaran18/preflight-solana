"use client";

import { useEffect, useState, type MouseEvent } from "react";
import { flushSync } from "react-dom";
import { cx } from "@/lib/format";
import { MoonIcon, SunIcon } from "./icons";

type Theme = "light" | "dark";

type TransitionDocument = Document & {
  startViewTransition?: (update: () => void) => { ready: Promise<void> };
};

const STORAGE_KEY = "preflight-theme";

function readTheme(): Theme {
  return document.documentElement.dataset.theme === "light" ? "light" : "dark";
}

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.dataset.theme = theme;
  root.style.colorScheme = theme;
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {}
}

export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme | null>(null);

  useEffect(() => {
    setTheme(readTheme());
  }, []);

  function toggle(event: MouseEvent<HTMLButtonElement>) {
    const next: Theme = readTheme() === "dark" ? "light" : "dark";
    const update = () => {
      applyTheme(next);
      setTheme(next);
    };
    const doc = document as TransitionDocument;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced || typeof doc.startViewTransition !== "function") {
      update();
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;
    const radius = Math.hypot(Math.max(x, window.innerWidth - x), Math.max(y, window.innerHeight - y));
    const transition = doc.startViewTransition(() => flushSync(update));
    void transition.ready.then(() => {
      document.documentElement.animate(
        { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`] },
        { duration: 560, easing: "cubic-bezier(0.16, 1, 0.3, 1)", pseudoElement: "::view-transition-new(root)" },
      );
    });
  }

  const label = theme === "dark" ? "Switch to light theme" : "Switch to dark theme";
  return (
    <button
      type="button"
      onClick={toggle}
      aria-label={label}
      title={label}
      className="relative grid h-9 w-9 shrink-0 place-items-center overflow-hidden rounded-xl border border-line bg-surface text-mute transition hover:border-line-strong hover:text-ink active:scale-95 sm:h-10 sm:w-10"
    >
      <SunIcon
        className={cx(
          "absolute h-4.5 w-4.5 transition duration-500 ease-out",
          theme === "light" ? "rotate-0 scale-100 opacity-100" : "-rotate-90 scale-50 opacity-0",
        )}
      />
      <MoonIcon
        className={cx(
          "absolute h-4.5 w-4.5 transition duration-500 ease-out",
          theme === "dark" ? "rotate-0 scale-100 opacity-100" : "rotate-90 scale-50 opacity-0",
        )}
      />
    </button>
  );
}
