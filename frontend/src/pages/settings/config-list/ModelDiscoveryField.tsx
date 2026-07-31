import { useEffect, useId, useState } from "react";
import { Radar } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import type { DiscoveredModel, ModelDiscoveryResult } from "../../../api/modelDiscovery";
import { useToast } from "../../../components/feedback/Toast";
import { BookInput } from "../../../components/ui/BookInput";

interface ModelDiscoveryFieldProps {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  disabled?: boolean;
  inputDisabled?: boolean;
  discoveryKey?: string;
  detectModels: () => Promise<ModelDiscoveryResult>;
}

export function ModelDiscoveryField({
  value,
  onChange,
  placeholder,
  disabled = false,
  inputDisabled = disabled,
  discoveryKey,
  detectModels,
}: ModelDiscoveryFieldProps) {
  const datalistId = useId();
  const addToast = useToast((state) => state.addToast);
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
      if (result.models.length === 0) {
        throw new Error("模型探测响应没有返回可用模型");
      }
      setModels(result.models);
      if (!value.trim() && result.models[0]?.id) {
        onChange(result.models[0].id);
      }
      addToast(`已发现 ${result.models.length} 个模型`, "success");
    } catch (err) {
      addToast(extractErrorMessage(err, "模型探测失败"), "error");
    } finally {
      setDetecting(false);
    }
  };

  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        <BookInput
          value={value}
          list={models.length > 0 ? datalistId : undefined}
          onChange={(event) => onChange(event.target.value)}
          placeholder={placeholder}
          disabled={inputDisabled}
        />
        <button
          type="button"
          className="pyqt-btn-secondary inline-flex min-h-[40px] shrink-0 items-center justify-center gap-1.5 px-3 text-xs disabled:cursor-not-allowed disabled:opacity-60"
          onClick={() => void handleDetect()}
          disabled={disabled || detecting}
          title="检测模型列表"
        >
          {detecting ? <LoadingSpinner size="sm" /> : <Radar size={13} />}
          <span>检测</span>
        </button>
      </div>
      {models.length > 0 ? (
        <datalist id={datalistId}>
          {models.map((model) => (
            <option key={model.id} value={model.id}>
              {modelOptionLabel(model)}
            </option>
          ))}
        </datalist>
      ) : null}
      {models.length > 0 ? (
        <div className="text-small text-book-text-muted">已加载 {models.length} 个模型</div>
      ) : null}
    </div>
  );
}

function modelOptionLabel(model: DiscoveredModel): string {
  const parts = [model.name, model.owned_by].filter(
    (part): part is string => typeof part === "string" && part.trim().length > 0,
  );
  return parts.length > 0 ? parts.join(" · ") : model.id;
}
