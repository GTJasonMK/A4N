import { useEffect, useId, useRef, useState } from "react";
import { Radar } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import type { DiscoveredModel, ModelDiscoveryResult } from "../../../api/modelDiscovery";
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
  const { t } = useI18n();
  const [models, setModels] = useState<DiscoveredModel[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [detecting, setDetecting] = useState(false);
  const feedbackId = useId();
  const requestRef = useRef<symbol | null>(null);
  const latestRef = useRef({ discoveryKey, value, inputDisabled, onChange });
  latestRef.current = { discoveryKey, value, inputDisabled, onChange };

  useEffect(() => {
    requestRef.current = null;
    setModels(null);
    setError(null);
    setDetecting(false);
    return () => { requestRef.current = null; };
  }, [discoveryKey]);

  const handleDetect = async () => {
    if (requestRef.current != null || disabled) return;
    const request = Symbol("model-discovery");
    requestRef.current = request;
    const isCurrent = () => requestRef.current === request && latestRef.current.discoveryKey === discoveryKey;
    setDetecting(true);
    setModels(null);
    setError(null);
    try {
      const result = await detectModels();
      if (!isCurrent()) return;
      setModels(result.models);
      const latest = latestRef.current;
      if (result.models.length > 0 && !latest.inputDisabled && !latest.value.trim()) {
        latest.onChange(result.models[0].id);
      }
    } catch (err) {
      if (isCurrent()) setError(extractErrorMessage(err, t("settings.modelDiscoveryFailed")));
    } finally {
      if (isCurrent()) {
        requestRef.current = null;
        setDetecting(false);
      }
    }
  };

  return (
    <div className="space-y-2" aria-busy={detecting}>
      <div className="flex gap-2">
        <ThemedCombobox
          value={value}
          options={(models ?? []).map((model) => ({
            value: model.id,
            label: modelOptionLabel(model),
          }))}
          onChange={onChange}
          placeholder={placeholder}
          disabled={inputDisabled}
          aria-describedby={feedbackId}
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
      <div id={feedbackId} role={error ? "alert" : "status"} className={`min-h-4 text-small ${error ? "text-error" : "text-book-text-muted"}`}>
        {error ?? (models == null ? null : models.length > 0
          ? t("settings.loadedModels", { count: models.length })
          : t("settings.noModelsFound"))}
      </div>
    </div>
  );
}

function modelOptionLabel(model: DiscoveredModel): string {
  const parts = [model.name, model.owned_by].filter(
    (part): part is string => part !== null,
  );
  return parts.length > 0 ? parts.join(" · ") : model.id;
}
