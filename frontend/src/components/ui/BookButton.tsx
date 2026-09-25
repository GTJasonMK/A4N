import React from "react";

interface BookButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "warning" | "danger";
  size?: "sm" | "md" | "lg";
}

export const BookButton: React.FC<BookButtonProps> = ({
  children,
  variant = "primary",
  size = "md",
  className = "",
  type,
  ...props
}) => {
  const sizes = {
    sm: "min-h-8 px-2.5 text-xs",
    md: "min-h-9 px-3.5 text-sm",
    lg: "min-h-11 px-5 text-base",
  } as const;

  const variants = {
    primary: "pyqt-btn-primary",
    secondary: "pyqt-btn-secondary",
    ghost: "a4n-btn-ghost",
    warning: "a4n-btn-warning",
    danger: "a4n-btn-danger",
  };

  return (
    <button
      type={type ?? "button"}
      className={`a4n-touch-target ${variants[variant]} ${sizes[size]} min-w-0 max-w-full whitespace-normal text-center sm:whitespace-nowrap ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};

interface BookIconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  variant?: "secondary" | "ghost";
  size?: "sm" | "md";
}

export const BookIconButton: React.FC<BookIconButtonProps> = ({
  children,
  label,
  variant = "ghost",
  size = "md",
  className = "",
  type,
  ...props
}) => {
  const sizes = {
    sm: "size-8 min-h-8",
    md: "size-9 min-h-9",
  } as const;

  const variants = {
    secondary: "pyqt-btn-icon",
    ghost: "a4n-btn-ghost",
  } as const;

  return (
    <button
      type={type ?? "button"}
      aria-label={label}
      title={props.title ?? label}
      className={`a4n-touch-target a4n-icon-button ${variants[variant]} ${sizes[size]} p-0 ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
