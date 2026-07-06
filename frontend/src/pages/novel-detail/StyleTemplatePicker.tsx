import React, { useEffect, useMemo, useState } from "react";
import { Palette } from "lucide-react";
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
    <div className="min-w-0 space-y-1.5 text-xs">
      <div className="flex items-center justify-between gap-2">
        <label className="inline-flex min-w-0 items-center gap-1.5 text-book-text-muted">
          <Palette size={12} />
          <span className="truncate">{label}</span>
        </label>
        <select
          value={selectedTemplate?.id ?? "custom"}
          onChange={(event) => applyTemplate(event.target.value)}
          disabled={disabled}
          title={selectedTemplate?.name ?? "自定义"}
          className="pyqt-input a4n-compact-select min-w-[10rem] max-w-[58%] text-xs"
        >
          <option value="custom">自定义</option>
          {templates.map((template) => (
            <option key={template.id} value={template.id}>
              {template.name}
            </option>
          ))}
        </select>
      </div>
      <textarea
        value={value}
        onChange={(event) => {
          setCustomMode(true);
          onChange(event.target.value);
        }}
        disabled={disabled}
        rows={rows}
        className="pyqt-input min-h-[88px] w-full resize-y !px-2 !py-1.5 leading-relaxed"
        placeholder="输入自定义风格，或从右上角选择内置模板"
      />
      {selectedTemplate ? (
        <details className="a4n-stat-tile p-2 text-[11px] text-book-text-sub">
          <summary className="cursor-pointer truncate text-book-text-muted">
            {selectedTemplate.summary}
          </summary>
          <div className="mt-1.5 grid grid-cols-1 gap-1 sm:grid-cols-2">
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
      ) : null}
    </div>
  );
};

const StyleField: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="min-w-0">
    <span className="text-book-text-muted">{label}：</span>
    <span>{value}</span>
  </div>
);
