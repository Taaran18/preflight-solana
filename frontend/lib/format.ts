export function cx(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(" ");
}

export function titleCase(text: string): string {
  return text
    .split(" ")
    .map((word) => word.split("-").map((part) => (/^[a-z]/.test(part) ? part[0].toUpperCase() + part.slice(1) : part)).join("-"))
    .join(" ");
}

export function formatAmount(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "–";
  const abs = Math.abs(value);
  const digits = abs >= 1000 ? 2 : abs >= 1 ? 4 : abs >= 0.0001 ? 6 : 9;
  return value.toLocaleString("en-US", { maximumFractionDigits: digits });
}

export function formatSigned(value: number): string {
  const sign = value > 0 ? "+" : value < 0 ? "−" : "";
  return `${sign}${formatAmount(Math.abs(value))}`;
}

export function formatUsd(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "–";
  const abs = Math.abs(value);
  return value.toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: abs >= 1 ? 2 : 0,
    maximumFractionDigits: abs >= 1 ? 2 : abs >= 0.01 ? 4 : 8,
  });
}

export function formatSignedUsd(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "";
  const sign = value > 0 ? "+" : value < 0 ? "−" : "";
  return `${sign}${formatUsd(Math.abs(value))}`;
}

export function formatCompactUsd(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "–";
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

export function formatCompact(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "–";
  return new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 1 }).format(value);
}

export function formatPercent(value: number | null | undefined, digits = 1): string {
  if (value == null || !Number.isFinite(value)) return "–";
  return `${value.toFixed(digits)}%`;
}

export function formatAge(days: number | null | undefined): string {
  if (days == null || !Number.isFinite(days)) return "–";
  if (days < 1) return `${Math.max(1, Math.round(days * 24))}h`;
  if (days < 60) return `${Math.round(days)}d`;
  if (days < 730) return `${Math.round(days / 30)}mo`;
  return `${(days / 365).toFixed(1)}y`;
}

export function shortAddress(value: string, size = 4): string {
  return value.length <= size * 2 + 1 ? value : `${value.slice(0, size)}…${value.slice(-size)}`;
}

export function isAddress(value: string): boolean {
  return /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(value.trim());
}

export function explorerTx(signature: string): string {
  return `https://solscan.io/tx/${signature}`;
}

export function explorerAccount(address: string): string {
  return `https://solscan.io/account/${address}`;
}

export function explorerToken(mint: string): string {
  return `https://solscan.io/token/${mint}`;
}
