import React from "react";

interface SwitchTrackProps {
  checked: boolean;
  size?: "sm" | "md";
  tone?: "primary" | "danger";
  className?: string;
}

export const SwitchTrack: React.FC<SwitchTrackProps> = ({
  checked,
  size = "sm",
  tone = "primary",
  className = "",
}) => {
  const trackSize = size === "md" ? "h-6 w-11" : "h-[18px] w-8";
  const thumbSize =
    size === "md"
      ? checked
        ? "top-0.5 h-5 w-5 translate-x-[22px]"
        : "top-0.5 h-5 w-5 translate-x-0.5"
      : checked
        ? "top-[2px] h-3.5 w-3.5 translate-x-4"
        : "top-[2px] h-3.5 w-3.5 translate-x-[2px]";
  const activeTone = tone === "danger" ? "bg-error/80" : "bg-book-primary";

  return (
    <span
      className={`pointer-events-none relative inline-flex shrink-0 rounded-full transition-colors ${trackSize} ${
        checked ? activeTone : "bg-book-text-muted/25"
      } ${className}`.trim()}
      aria-hidden="true"
    >
      <span className={`absolute rounded-full bg-white shadow-sm transition-transform ${thumbSize}`} />
    </span>
  );
};
