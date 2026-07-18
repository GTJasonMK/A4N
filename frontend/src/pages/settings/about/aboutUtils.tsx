import React from "react";

export const Row: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <div className="grid grid-cols-[auto_minmax(0,1fr)] items-start gap-x-3 gap-y-0.5">
    <dt className="shrink-0 text-book-text-muted">{label}</dt>
    <dd className="min-w-0 text-right text-book-text-main">{children}</dd>
  </div>
);
