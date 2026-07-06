import React from "react";
import { Loader2 } from "lucide-react";

interface MetricCardProps {
  label: string;
  value: number | undefined;
  tone: "muted" | "active" | "success" | "danger";
  loading: boolean;
}

export const MetricCard: React.FC<MetricCardProps> = ({ label, value, tone, loading }) => {
  const toneClass = {
    muted: "text-book-text-muted",
    active: "text-book-primary",
    success: "text-success",
    danger: "text-error",
  }[tone];

  return (
    <div className="pyqt-card p-4 flex flex-col items-start">
      <span className="text-xs text-book-text-muted mb-1">{label}</span>
      <span className={`text-3xl font-bold font-mono ${toneClass}`}>
        {loading ? (
          <Loader2 size={20} className="animate-spin" />
        ) : value == null ? (
          <span className="text-sm font-sans font-normal text-book-text-muted">未返回</span>
        ) : (
          value.toLocaleString()
        )}
      </span>
    </div>
  );
};
