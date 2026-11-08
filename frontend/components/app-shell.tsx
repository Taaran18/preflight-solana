"use client";

import dynamic from "next/dynamic";
import { useCallback, useEffect, useRef, useState } from "react";
import { useWallet } from "@solana/wallet-adapter-react";
import { API_URL, DEMO_WALLET, getJson } from "@/lib/api";
import { cx, isAddress } from "@/lib/format";
import type { Activity, Health, Portfolio } from "@/lib/types";
import { Copilot } from "./copilot";
import { Inspector } from "./inspector";
import { ActivityPanel, HowItWorks, MarketPanel, PortfolioPanel } from "./sidebar";
import { ThemeToggle } from "./theme";
import { buttonClass } from "./ui";
import { Logo, RefreshIcon, SearchIcon, SparkIcon, XCircleIcon } from "./icons";

const WalletButton = dynamic(
  () => import("@solana/wallet-adapter-react-ui").then((mod) => mod.WalletMultiButton),
  { ssr: false, loading: () => <div className="skeleton h-10 w-36 rounded-xl" /> },
);

type Tab = "copilot" | "inspect";

const TABS: { id: Tab; label: string; icon: typeof SparkIcon }[] = [
  { id: "copilot", label: "Copilot", icon: SparkIcon },
  { id: "inspect", label: "Inspect", icon: SearchIcon },
];

export function AppShell() {
  const { publicKey } = useWallet();
  const [tab, setTab] = useState<Tab>("copilot");
  const [leaving, setLeaving] = useState(false);
  const [watch, setWatch] = useState("");
  const [health, setHealth] = useState<Health | null>(null);
  const [offline, setOffline] = useState<string | null>(null);
  const [portfolio, setPortfolio] = useState<Portfolio | null>(null);
  const [portfolioError, setPortfolioError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [activity, setActivity] = useState<Activity[]>([]);
  const [draft, setDraft] = useState<{ text: string; nonce: number } | null>(null);
  const tabTimer = useRef<number | undefined>(undefined);
  const requestId = useRef(0);

  const connectedAddress = publicKey?.toBase58() ?? null;
  const owner = connectedAddress ?? (isAddress(watch) ? watch.trim() : null);

  const checkHealth = useCallback(() => {
    getJson<Health>("/api/health")
      .then((value) => {
        setHealth(value);
        setOffline(null);
      })
      .catch((err: unknown) => setOffline(err instanceof Error ? err.message : String(err)));
  }, []);

  useEffect(() => {
    checkHealth();
  }, [checkHealth]);

  const refreshPortfolio = useCallback(
    async (fresh = false) => {
      const id = ++requestId.current;
      if (!owner) {
        setPortfolio(null);
        setPortfolioError(null);
        setLoading(false);
        return;
      }
      setLoading(true);
      try {
        const value = await getJson<Portfolio>(
          `/api/portfolio?owner=${encodeURIComponent(owner)}${fresh ? "&refresh=true" : ""}`,
        );
        if (id !== requestId.current) return;
        setPortfolio(value.owner === owner ? value : null);
        setPortfolioError(null);
      } catch (err) {
        if (id !== requestId.current) return;
        setPortfolioError(err instanceof Error ? err.message : String(err));
      } finally {
        if (id === requestId.current) setLoading(false);
      }
    },
    [owner],
  );

  useEffect(() => {
    setPortfolio(null);
    void refreshPortfolio();
  }, [refreshPortfolio]);

  useEffect(() => () => window.clearTimeout(tabTimer.current), []);

  const switchTab = useCallback(
    (next: Tab) => {
      if (next === tab || leaving) return;
      setLeaving(true);
      window.clearTimeout(tabTimer.current);
      tabTimer.current = window.setTimeout(() => {
        setTab(next);
        setLeaving(false);
        window.scrollTo({ top: 0 });
      }, 170);
    },
    [tab, leaving],
  );

  const ask = useCallback((text: string) => {
    setTab("copilot");
    setLeaving(false);
    setDraft({ text, nonce: Date.now() });
    window.scrollTo({ top: document.body.scrollHeight, behavior: "smooth" });
  }, []);

  const startDemo = useCallback(() => setWatch(DEMO_WALLET), []);

  const onExecuted = useCallback(
    (item: Activity) => {
      setActivity((current) => [item, ...current]);
      window.setTimeout(() => void refreshPortfolio(true), 1200);
    },
    [refreshPortfolio],
  );

  const panelClass = (id: Tab) => cx(tab !== id && "hidden", tab === id && (leaving ? "animate-exit" : "animate-enter"));

  return (
    <div className="flex min-h-dvh w-full flex-col px-4 sm:px-6 lg:h-dvh lg:overflow-hidden lg:px-8 2xl:px-12">
      <header className="sticky top-0 z-30 -mx-4 flex h-14 items-center gap-2 border-b border-line bg-bg/85 px-4 backdrop-blur-xl sm:-mx-6 sm:h-16 sm:gap-3 sm:px-6 lg:-mx-8 lg:px-8 2xl:-mx-12 2xl:px-12">
        <div className="flex shrink-0 items-center gap-2.5">
          <Logo className="h-8 w-8" />
          <div className="leading-tight max-[420px]:hidden">
            <div className="text-[15px] font-semibold tracking-tight text-ink">Preflight</div>
            <div className="hidden text-[11px] text-faint sm:block">AI copilot for Solana</div>
          </div>
        </div>
        <nav aria-label="Sections" className="relative ml-2 flex rounded-xl border border-line bg-surface-2 p-1 sm:ml-6">
          <span
            aria-hidden="true"
            className={cx(
              "absolute inset-y-1 left-1 w-[calc(50%-4px)] rounded-lg bg-surface shadow-sm ring-1 ring-line transition-transform duration-300 ease-out",
              tab === "inspect" && "translate-x-full",
            )}
          />
          {TABS.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              onClick={() => switchTab(id)}
              aria-current={tab === id ? "page" : undefined}
              className={cx(
                "relative z-10 inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-3 py-1.5 text-sm font-medium transition-colors duration-300 sm:min-w-28",
                tab === id ? "text-ink" : "text-mute hover:text-ink",
              )}
            >
              <Icon className={cx("h-4 w-4", tab === id && "text-primary")} />
              <span className="hidden sm:inline">{label}</span>
            </button>
          ))}
        </nav>
        <div className="ml-auto flex items-center gap-2">
          <ThemeToggle />
          <WalletButton />
        </div>
      </header>

      {offline && (
        <div className="mt-4 flex animate-enter flex-col gap-3 rounded-2xl border border-bad-line bg-bad-soft px-4 py-3 text-sm sm:flex-row sm:items-start">
          <XCircleIcon className="mt-0.5 hidden h-5 w-5 shrink-0 text-bad sm:block" />
          <div className="flex-1">
            <div className="font-semibold text-bad">The Preflight API Isn&apos;t Reachable</div>
            <div className="mt-0.5 text-mute">
              Expected it at <code className="font-mono text-ink">{API_URL}</code>. Run <code className="font-mono text-ink">dev.bat</code>{" "}
              (Windows) or <code className="font-mono text-ink">./dev.sh</code> from the project root.
            </div>
          </div>
          <button type="button" onClick={checkHealth} className={buttonClass("secondary", "sm")}>
            <RefreshIcon className="h-3.5 w-3.5" />
            Retry
          </button>
        </div>
      )}

      <main className="grid flex-1 gap-6 pb-8 lg:min-h-0 lg:grid-cols-[minmax(0,1fr)_340px] lg:gap-8 lg:pb-0 xl:grid-cols-[minmax(0,1fr)_380px] 2xl:grid-cols-[minmax(0,1fr)_440px] 2xl:gap-12">
        <section className="min-w-0 lg:flex lg:min-h-0 lg:flex-col">
          <div className={cx(panelClass("copilot"), "lg:min-h-0 lg:flex-1")}>
            <Copilot
              owner={owner}
              connected={connectedAddress !== null}
              portfolio={portfolio}
              draft={draft}
              onExecuted={onExecuted}
              onAsk={ask}
              onDemo={owner ? null : startDemo}
            />
          </div>
          <div className={cx(panelClass("inspect"), "scroll-thin lg:min-h-0 lg:flex-1 lg:overflow-x-hidden lg:overflow-y-auto lg:pb-6 lg:pr-1")}>
            <Inspector owner={owner} />
          </div>
        </section>
        <aside className="scroll-thin grid content-start gap-4 md:grid-cols-2 lg:flex lg:min-h-0 lg:flex-col lg:overflow-x-hidden lg:overflow-y-auto lg:py-5 lg:pr-1">
          <PortfolioPanel
            owner={owner}
            connected={connectedAddress !== null}
            portfolio={portfolio}
            loading={loading}
            error={portfolioError}
            watch={watch}
            onWatch={setWatch}
            onRefresh={() => void refreshPortfolio(true)}
            onAsk={ask}
            onDemo={startDemo}
          />
          <ActivityPanel activity={activity} />
          <HowItWorks />
          <MarketPanel onAsk={ask} />
        </aside>
      </main>
    </div>
  );
}
