import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, { useEffect, useMemo, useState } from "react";
import { ChevronRight } from "lucide-react";
import type { VisualStyleTemplate } from "./styleTemplates";
import { renderVisualStyleTemplate } from "./styleTemplates";
import { useI18n, type MessageKey } from "../../i18n";

const STYLE_TEMPLATE_KEYS: Record<
  string,
  { name: MessageKey; summary: MessageKey }
> = {
  "cinematic-realism": {
    name: "common.stylePicker.templates.cinematicRealismName",
    summary: "common.stylePicker.templates.cinematicRealismSummary",
  },
  "anime-cel": {
    name: "common.stylePicker.templates.animeCelName",
    summary: "common.stylePicker.templates.animeCelSummary",
  },
  "kyoani-everyday-anime": {
    name: "common.stylePicker.templates.kyoaniEverydayAnimeName",
    summary: "common.stylePicker.templates.kyoaniEverydayAnimeSummary",
  },
  "fantasy-concept": {
    name: "common.stylePicker.templates.fantasyConceptName",
    summary: "common.stylePicker.templates.fantasyConceptSummary",
  },
  "watercolor-storybook": {
    name: "common.stylePicker.templates.watercolorStorybookName",
    summary: "common.stylePicker.templates.watercolorStorybookSummary",
  },
  "ink-oriental": {
    name: "common.stylePicker.templates.inkOrientalName",
    summary: "common.stylePicker.templates.inkOrientalSummary",
  },
  "noir-thriller": {
    name: "common.stylePicker.templates.noirThrillerName",
    summary: "common.stylePicker.templates.noirThrillerSummary",
  },
  "cyberpunk-neon": {
    name: "common.stylePicker.templates.cyberpunkNeonName",
    summary: "common.stylePicker.templates.cyberpunkNeonSummary",
  },
  "retro-film": {
    name: "common.stylePicker.templates.retroFilmName",
    summary: "common.stylePicker.templates.retroFilmSummary",
  },
};

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
  const { t } = useI18n();
  const matchedTemplate = useMemo(
    () => templates.find((template) => renderVisualStyleTemplate(template) === value) ?? null,
    [templates, value],
  );
  const [customMode, setCustomMode] = useState(matchedTemplate == null);
  const selectedTemplate = customMode ? null : matchedTemplate;
  const templateName = (template: VisualStyleTemplate): string => {
    const keys = STYLE_TEMPLATE_KEYS[template.id];
    return keys ? t(keys.name) : template.name;
  };
  const templateSummary = (template: VisualStyleTemplate): string => {
    const keys = STYLE_TEMPLATE_KEYS[template.id];
    return keys ? t(keys.summary) : template.summary;
  };

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
        <ThemedSelect
          value={selectedTemplate?.id ?? "custom"}
          onChange={(event) => applyTemplate(event.target.value)}
          disabled={disabled}
          title={selectedTemplate ? templateName(selectedTemplate) : t("common.stylePicker.custom")}
          aria-label={t("common.stylePicker.templateAria", { label })}
          className="pyqt-input w-full !py-1.5 text-xs sm:min-w-[10rem]"
        >
          <option value="custom">{t("common.stylePicker.custom")}</option>
          {templates.map((template) => (
            <option key={template.id} value={template.id}>
              {templateName(template)}
            </option>
          ))}
        </ThemedSelect>
      </div>

      {selectedTemplate ? (
        <div className="border-y border-book-border/45 py-2.5">
          <p className="text-small leading-relaxed text-book-text-muted">
            {templateSummary(selectedTemplate)}
          </p>
          <details className="group mt-1.5">
            <summary className="flex min-h-8 cursor-pointer list-none items-center gap-1.5 rounded-lg px-1.5 text-small text-book-text-muted transition-colors hover:bg-book-bg-tertiary/55 hover:text-book-text-main [&::-webkit-details-marker]:hidden">
              <ChevronRight
                size={12}
                className="shrink-0 transition-transform group-open:rotate-90"
              />
              {t("common.stylePicker.details")}
            </summary>
            <div className="mt-1 divide-y divide-book-border/35 border-y border-book-border/35">
              <StyleField label={t("common.stylePicker.styleNames")} value={selectedTemplate.fields.styleNames} />
              <StyleField label={t("common.stylePicker.tone")} value={selectedTemplate.fields.tone} />
              <StyleField label={t("common.stylePicker.lighting")} value={selectedTemplate.fields.lighting} />
              <StyleField label={t("common.stylePicker.color")} value={selectedTemplate.fields.color} />
              <StyleField label={t("common.stylePicker.composition")} value={selectedTemplate.fields.composition} />
              <StyleField label={t("common.stylePicker.camera")} value={selectedTemplate.fields.camera} />
              <StyleField label={t("common.stylePicker.material")} value={selectedTemplate.fields.material} />
              <StyleField label={t("common.stylePicker.atmosphere")} value={selectedTemplate.fields.atmosphere} />
              <StyleField label={t("common.stylePicker.detail")} value={selectedTemplate.fields.detail} />
              <StyleField label={t("common.stylePicker.motion")} value={selectedTemplate.fields.motion} />
              <StyleField label={t("common.stylePicker.avoid")} value={selectedTemplate.fields.avoid} />
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
          placeholder={t("common.stylePicker.customPlaceholder")}
          aria-label={t("common.stylePicker.customDescriptionAria", { label })}
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
