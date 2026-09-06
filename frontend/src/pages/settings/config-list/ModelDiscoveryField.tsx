import { useEffect, useState } from "react";
import { Radar } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import type { DiscoveredModel, ModelDiscoveryResult } from "../../../api/modelDiscovery";
import { useToast } from "../../../components/feedback/Toast";
import { ThemedCombobox } from "../../../components/ui/ThemedCombobox";
import { useI18n } from "../../../i18n";

interface ModelDiscoveryFieldProps {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  disabled: boolean;
  inputDisabled: boolean;
  discoveryKey: string;
  detectModels: () => Promise<ModelDiscoveryResult>;
}

export function ModelDiscoveryField({
  value,
  onChange,
  placeholder,
  disabled,
  inputDisabled,
  discoveryKey,
  detectModels,
}: ModelDiscoveryFieldProps) {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const [models, setModels] = useState<DiscoveredModel[]>([]);
  const [detecting, setDetecting] = useState(false);

  useEffect(() => {
    setModels([]);
  }, [discoveryKey]);

  const handleDetect = async () => {
    if (detecting || disabled) return;
    setDetecting(true);
    setModels([]);
    try {
      const result = await detectModels();
      setModels(result.models);
      if (!value.trim()) {
        onChange(result.models[0].id);
      }
      addToast(t("settings.discoveredModels", { count: result.models.length }), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("settings.modelDiscoveryFailed")), "error");
    } finally {
      setDetecting(false);
    }
  };

  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        <ThemedCombobox
          value={value}
          options={models.map((model) => ({
            value: model.id,
            label: modelOptionLabel(model),
          }))}
          onChange={onChange}
          placeholder={placeholder}
          disabled={inputDisabled}
        />
        <button
          type="button"
          className="pyqt-btn-secondary inline-flex min-h-[40px] shrink-0 items-center justify-center gap-1.5 px-3 text-xs disabled:cursor-not-allowed disabled:opacity-60"
          onClick={handleDetect}
          disabled={disabled || detecting}
          title={t("settings.detectModels")}
        >
          {detecting ? <LoadingSpinner size="sm" /> : <Radar size={13} />}
          <span>{t("settings.detect")}</span>
        </button>
      </div>
      {models.length > 0 ? (
        <div className="text-small text-book-text-muted">
          {t("settings.loadedModels", { count: models.length })}
        </div>
      ) : null}
    </div>
  );
}

function modelOptionLabel(model: DiscoveredModel): string {
  const parts = [model.name, model.owned_by].filter(
    (part): part is string => part !== null,
  );
  return parts.length > 0 ? parts.join(" · ") : model.id;
}
