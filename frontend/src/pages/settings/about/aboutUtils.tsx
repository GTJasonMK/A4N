import React from "react";

export function sessionDeviceLabel(userAgent: string | null): string {
  if (!userAgent) return "";
  const browser = ([
    [/Edg(?:e|A|iOS)?\//, "Edge"], [/Firefox\/|FxiOS\//, "Firefox"],
    [/Chrome\/|CriOS\//, "Chrome"], [/Safari\//, "Safari"], [/A4N Desktop/, "A4N Desktop"],
  ] as const).find(([pattern]) => pattern.test(userAgent))?.[1];
  const system = ([
    [/iPad/, "iPad"], [/iPhone/, "iPhone"], [/Android/, "Android"],
    [/Windows/, "Windows"], [/Macintosh|Mac OS X/, "macOS"], [/Linux/, "Linux"],
  ] as const).find(([pattern]) => pattern.test(userAgent))?.[1];
  return [browser, system].filter(Boolean).join(" · ") || userAgent;
}

export const Row: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <div className="grid grid-cols-[auto_minmax(0,1fr)] items-start gap-x-3 gap-y-0.5">
    <dt className="shrink-0 text-book-text-muted">{label}</dt>
    <dd className="min-w-0 text-right text-book-text-main">{children}</dd>
  </div>
);
