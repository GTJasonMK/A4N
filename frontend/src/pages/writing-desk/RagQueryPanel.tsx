import React from "react";
import { Search } from "lucide-react";
import { RagQuickPanel } from "./RagQuickPanel";

interface RagQueryPanelProps {
  projectId: number;
  chapterNumber: number | null;
}

export const RagQueryPanel: React.FC<RagQueryPanelProps> = ({
  projectId,
  chapterNumber,
}) => (
  <aside className="flex h-full min-w-0 flex-col overflow-hidden border-l border-book-border bg-book-bg-paper/40 p-2">
    <h3 className="text-xs font-semibold text-book-text-sub uppercase tracking-wider mb-2 px-1">
      <Search size={11} className="inline mr-1" />
      RAG 查询
    </h3>
    <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar">
      <RagQuickPanel projectId={projectId} chapterNumber={chapterNumber} />
    </div>
  </aside>
);
