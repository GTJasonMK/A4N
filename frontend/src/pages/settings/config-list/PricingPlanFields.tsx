import React from "react";
import type { PricingPlan } from "../../../api/generated/config";
import { BookInput } from "../../../components/ui/BookInput";
import { FormField } from "../ConfigListPanel";
import { useI18n } from "../../../i18n";

interface PricingPlanFieldsProps {
  value: PricingPlan;
  onChange: (value: PricingPlan) => void;
  readonly?: boolean;
}

export const PricingPlanFields: React.FC<PricingPlanFieldsProps> = ({
  value,
  onChange,
  readonly = false,
}) => {
  const { t } = useI18n();
  const set = (key: keyof PricingPlan, next: string) =>
    onChange({ ...value, [key]: next.trim() ? next : null });

  return (
    <details className="border-y border-book-border/35 py-2" open>
      <summary className="cursor-pointer text-sm font-semibold text-book-text-main">
        {t("settingsForm.pricingTitle")}
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
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <PricingInput
            label={t("settingsForm.pricingLlmInput")}
            value={value.llm_input_per_million}
            onChange={(next) => set("llm_input_per_million", next)}
            disabled={readonly}
          />
          <PricingInput
            label={t("settingsForm.pricingLlmOutput")}
            value={value.llm_output_per_million}
            onChange={(next) => set("llm_output_per_million", next)}
            disabled={readonly}
          />
          <PricingInput
            label={t("settingsForm.pricingCacheRead")}
            value={value.llm_cache_read_per_million}
            onChange={(next) => set("llm_cache_read_per_million", next)}
            disabled={readonly}
          />
          <PricingInput
            label={t("settingsForm.pricingCacheWrite")}
            value={value.llm_cache_write_per_million}
            onChange={(next) => set("llm_cache_write_per_million", next)}
            disabled={readonly}
          />
          <PricingInput
            label={t("settingsForm.pricingEmbedding")}
            value={value.embedding_input_per_million}
            onChange={(next) => set("embedding_input_per_million", next)}
            disabled={readonly}
          />
          <PricingInput
            label={t("settingsForm.pricingImage")}
            value={value.image_per_generation}
            onChange={(next) => set("image_per_generation", next)}
            disabled={readonly}
          />
          <PricingInput
            label={t("settingsForm.pricingVideo")}
            value={value.video_per_second}
            onChange={(next) => set("video_per_second", next)}
            disabled={readonly}
          />
        </div>
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
