import React, { type ReactNode } from "react";

interface FormFieldProps {
  label: string;
  hint?: string;
  required?: boolean;
  children: ReactNode;
}

export const FormField: React.FC<FormFieldProps> = ({
  label,
  hint,
  required,
  children,
}) => (
  <label className="flex min-w-0 flex-col gap-1.5">
    <span className="text-xs font-semibold text-book-text-sub flex items-center gap-1">
      {label}
      {required && <span className="text-error">*</span>}
    </span>
    {children}
    {hint && <span className="text-small text-book-text-muted">{hint}</span>}
  </label>
);
