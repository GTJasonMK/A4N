import React from "react";

export const Row: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <div className="flex justify-between gap-3">
    <dt className="text-book-text-muted shrink-0">{label}</dt>
    <dd className="text-book-text-main text-right">{children}</dd>
  </div>
);
