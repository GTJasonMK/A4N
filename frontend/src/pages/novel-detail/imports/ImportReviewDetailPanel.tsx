import React from "react";
import { Loader2 } from "lucide-react";
import type { ImportReview, ImportReviewStatus } from "../../../api/import";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { Row } from "../shared";

interface ImportReviewDetailPanelProps {
  activeReview: ImportReview | null;
  activeReviewJson: string;
  activeReviewJsonError: string | null;
  actBusy: string | null;
  onAnalysisJsonChange: (value: string) => void;
  onModify: () => void | Promise<void>;
  onUpdateStatus: (id: number, status: ImportReviewStatus) => void | Promise<void>;
}

const IMPORT_REVIEW_STATUS_OPTIONS: ImportReviewStatus[] = [
  "pending",
  "analyzing",
  "reviewing",
  "accepted",
  "rejected",
  "modified",
];

export const ImportReviewDetailPanel: React.FC<ImportReviewDetailPanelProps> = ({
  activeReview,
  activeReviewJson,
  activeReviewJsonError,
  actBusy,
  onAnalysisJsonChange,
  onModify,
  onUpdateStatus,
}) => {
  const actionBusy = actBusy !== null;

  return (
    <BookCard className="p-3 h-fit sticky top-2">
      <h3 className="text-sm font-serif text-book-text-main mb-2">审核详情</h3>
      {activeReview == null ? (
        <div className="text-xs text-book-text-muted italic">点击列表的“详情”查看</div>
      ) : (
        <div className="space-y-2 text-xs">
          <div className="text-book-text-muted">
            #{activeReview.id} · {activeReview.filename}
          </div>
          <div className="grid grid-cols-2 gap-1 text-[11px]">
            <Row label="状态">{activeReview.review_status}</Row>
            <Row label="类型">{activeReview.file_type}</Row>
            <Row label="编码">{activeReview.detected_encoding || "—"}</Row>
            <Row label="章节数">{activeReview.parsed_chapters}</Row>
            <Row label="去重段">{activeReview.dedup_paragraphs}</Row>
            <Row label="去重率">
              {activeReview.dedup_rate != null
                ? `${(activeReview.dedup_rate * 100).toFixed(1)}%`
                : "—"}
            </Row>
          </div>
          <div>
            <div className="text-book-text-muted mb-1">分析结果（可编辑 JSON）</div>
            <textarea
              value={activeReviewJson}
              disabled={actionBusy}
              onChange={(e) => onAnalysisJsonChange(e.target.value)}
              className="pyqt-input w-full font-mono text-[10px] h-40 resize-y"
            />
            {activeReviewJsonError && (
              <div className="text-[10px] text-error mt-1">{activeReviewJsonError}</div>
            )}
            <div className="flex justify-end mt-1 gap-1">
              <BookButton variant="secondary" onClick={() => void onModify()} disabled={actionBusy}>
                {actBusy?.startsWith("modify-") ? (
                  <Loader2 size={11} className="animate-spin" />
                ) : (
                  "保存修改"
                )}
              </BookButton>
              <select
                className="pyqt-input !py-1 !px-2 text-[10px]"
                defaultValue=""
                disabled={actionBusy}
                onChange={(e) => {
                  const status = e.target.value as ImportReviewStatus;
                  if (status) void onUpdateStatus(activeReview.id, status);
                  e.currentTarget.value = "";
                }}
              >
                <option value="" disabled>
                  更新状态…
                </option>
                {IMPORT_REVIEW_STATUS_OPTIONS.map((status) => (
                  <option key={status} value={status}>
                    {status}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>
      )}
    </BookCard>
  );
};
