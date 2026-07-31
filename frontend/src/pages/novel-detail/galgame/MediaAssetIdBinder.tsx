import React, { useState } from "react";
import { Images } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";

/**
 * 素材编号绑定器
 *
 * 从已有素材库中通过编号绑定媒体资源，支持键盘回车提交与错误提示。
 */

interface MediaAssetIdBinderProps {
  label: string;
  disabled: boolean;
  onBind: (mediaAssetId: number) => void;
}

function parsePositiveInteger(value: string): number | null {
  const trimmed = value.trim();
  if (!/^[1-9]\d*$/.test(trimmed)) return null;
  const parsed = Number(trimmed);
  return Number.isSafeInteger(parsed) ? parsed : null;
}

export const MediaAssetIdBinder: React.FC<MediaAssetIdBinderProps> = ({
  label,
  disabled,
  onBind,
}) => {
  const [draft, setDraft] = useState("");
  const [error, setError] = useState("");
  const mediaAssetId = parsePositiveInteger(draft);

  const submit = () => {
    if (mediaAssetId == null) {
      setError("请输入有效的素材编号");
      return;
    }
    setError("");
    onBind(mediaAssetId);
  };

  return (
    <details className="border-t border-book-border/55 pt-2">
      <summary className="cursor-pointer list-none text-small text-book-text-muted hover:text-book-text-main [&::-webkit-details-marker]:hidden">
        从已有素材绑定
      </summary>
      <div className="mt-2 space-y-1">
        <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
          <input
            className="min-w-0 border border-book-border bg-book-bg px-2 py-1 text-xs text-book-text-main outline-none focus:border-book-primary"
            value={draft}
            disabled={disabled}
            inputMode="numeric"
            aria-label="素材编号"
            placeholder="素材编号"
            onChange={(event) => {
              setDraft(event.target.value);
              if (error) setError("");
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") submit();
            }}
          />
          <BookButton
            variant="secondary"
            size="sm"
            onClick={submit}
            disabled={disabled || mediaAssetId == null}
          >
            <Images size={13} />
            {label}
          </BookButton>
        </div>
        {error ? <div className="text-small text-error">{error}</div> : null}
      </div>
    </details>
  );
};
