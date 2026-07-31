import React, { useEffect, useMemo, useState } from "react";
import { ChevronRight } from "lucide-react";
import type { VisualStyleTemplate } from "./styleTemplates";
import { renderVisualStyleTemplate } from "./styleTemplates";

interface StyleTemplatePickerProps {
  label: string;
  value: string;
  templates: VisualStyleTemplate[];
  disabled?: boolean;
  rows?: number;
  onChange: (value: string) => void;
}

export const StyleTemplatePicker: React.FC<StyleTemplatePickerProps> = ({
  label,
  value,
  templates,
  disabled = false,
  rows = 4,
  onChange,
}) => {
  const matchedTemplate = useMemo(
    () => templates.find((template) => renderVisualStyleTemplate(template) === value) ?? null,
    [templates, value],
  );
  const [customMode, setCustomMode] = useState(matchedTemplate == null);
  const selectedTemplate = customMode ? null : matchedTemplate;

  useEffect(() => {
    setCustomMode(matchedTemplate == null);
  }, [matchedTemplate]);

  const applyTemplate = (templateId: string) => {
    if (templateId === "custom") {
      setCustomMode(true);
      return;
    }
    const template = templates.find((item) => item.id === templateId);
    if (template) {
      setCustomMode(false);
      onChange(renderVisualStyleTemplate(template));
    }
  };

  return (
    <div className="min-w-0 space-y-2 text-xs">
      <div className="grid gap-1.5 sm:grid-cols-[minmax(0,1fr)_minmax(10rem,auto)] sm:items-center">
        <label className="truncate text-small font-medium text-book-text-sub">
          {label}
        </label>
        <select
          value={selectedTemplate?.id ?? "custom"}
          onChange={(event) => applyTemplate(event.target.value)}
          disabled={disabled}
          title={selectedTemplate?.name ?? "自定义风格"}
          aria-label={`${label}模板`}
          className="pyqt-input w-full !py-1.5 text-xs sm:min-w-[10rem]"
        >
          <option value="custom">自定义风格</option>
          {templates.map((template) => (
            <option key={template.id} value={template.id}>
              {template.name}
            </option>
          ))}
        </select>
      </div>

      {selectedTemplate ? (
        <div className="border-y border-book-border/45 py-2.5">
          <p className="text-small leading-relaxed text-book-text-muted">
            {selectedTemplate.summary}
          </p>
          <details className="group mt-1.5">
            <summary className="flex min-h-8 cursor-pointer list-none items-center gap-1.5 rounded-lg px-1.5 text-small text-book-text-muted transition-colors hover:bg-book-bg-tertiary/55 hover:text-book-text-main [&::-webkit-details-marker]:hidden">
              <ChevronRight
                size={12}
                className="shrink-0 transition-transform group-open:rotate-90"
              />
              查看风格细节
            </summary>
            <div className="mt-1 divide-y divide-book-border/35 border-y border-book-border/35">
              <StyleField label="关键词" value={selectedTemplate.fields.styleNames} />
              <StyleField label="整体" value={selectedTemplate.fields.tone} />
              <StyleField label="光影" value={selectedTemplate.fields.lighting} />
              <StyleField label="色彩" value={selectedTemplate.fields.color} />
              <StyleField label="构图" value={selectedTemplate.fields.composition} />
              <StyleField label="镜头" value={selectedTemplate.fields.camera} />
              <StyleField label="材质" value={selectedTemplate.fields.material} />
              <StyleField label="氛围" value={selectedTemplate.fields.atmosphere} />
              <StyleField label="细节" value={selectedTemplate.fields.detail} />
              <StyleField label="运动" value={selectedTemplate.fields.motion} />
              <StyleField label="避免" value={selectedTemplate.fields.avoid} />
            </div>
          </details>
        </div>
      ) : (
        <textarea
          value={value}
          onChange={(event) => onChange(event.target.value)}
          disabled={disabled}
          rows={rows}
          className="pyqt-input min-h-[88px] w-full resize-y !px-2.5 !py-2 leading-relaxed"
          placeholder="描述期望的画面风格、光影、色彩、构图和需要避免的效果。"
          aria-label={`${label}自定义说明`}
        />
      )}
    </div>
  );
};

const StyleField: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="grid min-w-0 gap-1 py-2 sm:grid-cols-[3.5rem_minmax(0,1fr)] sm:gap-2">
    <span className="text-book-text-muted">{label}</span>
    <span className="leading-relaxed text-book-text-sub">{value}</span>
  </div>
);
