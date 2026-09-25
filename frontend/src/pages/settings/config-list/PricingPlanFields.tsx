import React from "react";
import type { PricingPlan } from "../../../api/generated/config";
import { BookInput } from "../../../components/ui/BookInput";
import { FormField } from "../ConfigListPanel";
import { useI18n } from "../../../i18n";

interface PricingPlanFieldsProps {
  kind: "llm" | "image" | "video" | "embedding";
  value: PricingPlan;
  onChange: (value: PricingPlan) => void;
  readonly?: boolean;
}

const pricingFields = [
  ["llm", "llm_input_per_million", "settingsForm.pricingLlmInput"],
  ["llm", "llm_output_per_million", "settingsForm.pricingLlmOutput"],
  ["llm", "llm_cache_read_per_million", "settingsForm.pricingCacheRead"],
  ["llm", "llm_cache_write_per_million", "settingsForm.pricingCacheWrite"],
  ["embedding", "embedding_input_per_million", "settingsForm.pricingEmbedding"],
  ["image", "image_per_generation", "settingsForm.pricingImage"],
  ["video", "video_per_second", "settingsForm.pricingVideo"],
] as const;

export const PricingPlanFields: React.FC<PricingPlanFieldsProps> = ({
  kind,
  value,
  onChange,
  readonly = false,
}) => {
  const { t } = useI18n();
  const set = (key: keyof PricingPlan, next: string) =>
    onChange({ ...value, [key]: next.trim() ? next : null });
  const fields = (primary: boolean) => pricingFields
    .filter(([group]) => (group === kind) === primary)
    .map(([, key, label]) => (
      <PricingInput key={key} label={t(label)} value={value[key]} onChange={(next) => set(key, next)} disabled={readonly} />
    ));

  return (
    <details className="border-y border-book-border/35 py-2">
      <summary className="cursor-pointer text-sm font-semibold text-book-text-main">
        {t("settingsForm.pricingTitle")}
        <span className="ml-2 text-xs font-normal text-book-text-muted">{value.currency}</span>
      </summary>
      <div className="mt-3 space-y-3">
        <FormField label={t("settingsForm.pricingCurrency")} hint={t("settingsForm.pricingCurrencyHint")}>
          <BookInput
            value={value.currency}
            maxLength={3}
            onChange={(event) =>
              onChange({ ...value, currency: event.target.value.toUpperCase() })
            }
            disabled={readonly}
          />
        </FormField>
        <div className="a4n-config-form-grid">
          {fields(true)}
        </div>
        <details>
          <summary className="cursor-pointer text-xs text-book-text-sub">{t("settingsForm.pricingOtherCapabilities")}</summary>
          <div className="a4n-config-form-grid mt-3">{fields(false)}</div>
        </details>
      </div>
    </details>
  );
};

const PricingInput: React.FC<{
  label: string;
  value: string | null;
  onChange: (value: string) => void;
  disabled: boolean;
}> = ({ label, value, onChange, disabled }) => (
  <FormField label={label}>
    <BookInput
      type="text"
      inputMode="decimal"
      value={value ?? ""}
      placeholder="0"
      onChange={(event) => onChange(event.target.value)}
      disabled={disabled}
    />
  </FormField>
);
