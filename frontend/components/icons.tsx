import type { ReactNode, SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

function Icon({ children, ...props }: IconProps & { children: ReactNode }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.9}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      {children}
    </svg>
  );
}

export function Logo(props: IconProps) {
  return (
    <svg viewBox="0 0 32 32" aria-hidden="true" {...props}>
      <rect width="32" height="32" rx="9" className="fill-primary" />
      <path
        d="M16 6.5l7.5 3v5.4c0 4.6-3.1 8.4-7.5 9.6-4.4-1.2-7.5-5-7.5-9.6V9.5l7.5-3z"
        fill="none"
        stroke="#fff"
        strokeWidth="2.2"
        strokeLinejoin="round"
      />
      <path d="M12.4 15.6l2.5 2.5 4.8-4.9" fill="none" stroke="#fff" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function ShieldIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 3l7.5 3v5.6c0 4.6-3.1 8.3-7.5 9.4-4.4-1.1-7.5-4.8-7.5-9.4V6L12 3z" />
    </Icon>
  );
}

export function ShieldCheckIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 3l7.5 3v5.6c0 4.6-3.1 8.3-7.5 9.4-4.4-1.1-7.5-4.8-7.5-9.4V6L12 3z" />
      <path d="M8.8 12.2l2.2 2.2 4.3-4.4" />
    </Icon>
  );
}

export function CheckIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M5 12.5l4.5 4.5L19 7.5" />
    </Icon>
  );
}

export function AlertIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M10.3 4.2L2.6 17.6A2 2 0 004.3 20.6h15.4a2 2 0 001.7-3L13.7 4.2a2 2 0 00-3.4 0z" />
      <path d="M12 9.5v4" />
      <path d="M12 17h.01" />
    </Icon>
  );
}

export function XCircleIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="9" />
      <path d="M15 9l-6 6M9 9l6 6" />
    </Icon>
  );
}

export function InfoIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5" />
      <path d="M12 7.6h.01" />
    </Icon>
  );
}

export function ArrowRightIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M5 12h14M13 6l6 6-6 6" />
    </Icon>
  );
}

export function ArrowDownIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 5v14M6 13l6 6 6-6" />
    </Icon>
  );
}

export function TrendUpIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M7 17L17 7M9 7h8v8" />
    </Icon>
  );
}

export function TrendDownIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M7 7l10 10M17 9v8H9" />
    </Icon>
  );
}

export function SparkIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9L12 3z" />
      <path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8L19 15z" />
    </Icon>
  );
}

export function CopyIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="9" y="9" width="11" height="11" rx="2" />
      <path d="M5 15V6a2 2 0 012-2h9" />
    </Icon>
  );
}

export function ExternalIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M14 4h6v6" />
      <path d="M20 4l-8.5 8.5" />
      <path d="M18 13.5V19a1 1 0 01-1 1H5a1 1 0 01-1-1V7a1 1 0 011-1h5.5" />
    </Icon>
  );
}

export function SendIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M21 3L10.5 13.5" />
      <path d="M21 3l-6.5 18-4-7.5L3 9.5 21 3z" />
    </Icon>
  );
}

export function SearchIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="11" cy="11" r="7" />
      <path d="M20 20l-3.6-3.6" />
    </Icon>
  );
}

export function WalletIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 7.5A2.5 2.5 0 016.5 5H18a2 2 0 012 2v1" />
      <rect x="3.5" y="7.5" width="17" height="12" rx="2.5" />
      <path d="M16 13.5h1.5" />
    </Icon>
  );
}

export function BoltIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M13 2.5L4.5 13.5h6.5l-1 8 8.5-11h-6.5l1-8z" />
    </Icon>
  );
}

export function RefreshIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M20 12a8 8 0 11-2.4-5.7" />
      <path d="M20 4.5v4.5h-4.5" />
    </Icon>
  );
}

export function LockIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="4.5" y="10.5" width="15" height="10" rx="2.5" />
      <path d="M8 10.5V8a4 4 0 018 0v2.5" />
    </Icon>
  );
}

export function ChevronIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M6 9l6 6 6-6" />
    </Icon>
  );
}

export function CodeIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M8.5 7L3.5 12l5 5M15.5 7l5 5-5 5" />
    </Icon>
  );
}

export function MessageIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M20 12.5a7.5 7.5 0 01-11.2 6.5L4 20l1.1-4.4A7.5 7.5 0 1120 12.5z" />
    </Icon>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M6 6l12 12M18 6L6 18" />
    </Icon>
  );
}

export function SunIcon(props: IconProps) {
  return (
    <Icon strokeWidth={2} {...props}>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2" />
      <path d="M12 20v2" />
      <path d="m4.93 4.93 1.41 1.41" />
      <path d="m17.66 17.66 1.41 1.41" />
      <path d="M2 12h2" />
      <path d="M20 12h2" />
      <path d="m6.34 17.66-1.41 1.41" />
      <path d="m19.07 4.93-1.41 1.41" />
    </Icon>
  );
}

export function MoonIcon(props: IconProps) {
  return (
    <Icon strokeWidth={2} {...props}>
      <path d="M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401" />
    </Icon>
  );
}

export function PlusIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 5v14M5 12h14" />
    </Icon>
  );
}

export function ActivityIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
    </Icon>
  );
}

export function ClockIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7.5V12l3 2" />
    </Icon>
  );
}

export function LayersIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 3l9 5-9 5-9-5 9-5z" />
      <path d="M3 13l9 5 9-5" />
    </Icon>
  );
}

export function SwapIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M7 4L3.5 7.5 7 11" />
      <path d="M3.5 7.5H17" />
      <path d="M17 13l3.5 3.5L17 20" />
      <path d="M20.5 16.5H7" />
    </Icon>
  );
}

export function CoinIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M14.5 9.2c-.5-.8-1.4-1.2-2.5-1.2-1.5 0-2.6.8-2.6 2s1.1 1.7 2.6 2 2.6.8 2.6 2-1.1 2-2.6 2c-1.1 0-2-.4-2.5-1.2M12 6.5v11" />
    </Icon>
  );
}
