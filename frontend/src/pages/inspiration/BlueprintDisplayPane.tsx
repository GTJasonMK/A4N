import React from "react";
import { Loader2, Sparkles } from "lucide-react";
import { BookCard } from "../../components/ui/BookCard";
import type { Blueprint } from "../../api/blueprint";
import { Section } from "./shared";

export const BlueprintDisplayPane: React.FC<{ blueprint: Blueprint }> = ({ blueprint }) => (
  <div className="h-full overflow-y-auto custom-scrollbar px-4 py-6">
    <div className="max-w-3xl mx-auto text-center">
      <div className="inline-flex items-center justify-center w-10 h-10 border border-book-primary/40 bg-book-primary/10 text-book-primary mb-3 rounded-sm">
        <Sparkles size={16} />
      </div>
      <h1 className="text-base font-serif font-semibold text-book-text-main mb-1">蓝图已就位</h1>
      <p className="text-xs text-book-text-muted mb-4">即将跳转到项目详情</p>
      <BookCard className="p-4 text-left">
        <Section label="风格">{blueprint.style || "—"}</Section>
        <div className="story-divider my-3" />
        <Section label="完整梗概">
          <div className="whitespace-pre-wrap leading-relaxed line-clamp-6">
            {blueprint.full_synopsis || "—"}
          </div>
        </Section>
      </BookCard>
      <div className="mt-3 text-xs text-book-text-muted inline-flex items-center gap-1">
        <Loader2 size={11} className="animate-spin" /> 跳转中…
      </div>
    </div>
  </div>
);
