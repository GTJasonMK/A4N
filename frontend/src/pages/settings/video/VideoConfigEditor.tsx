import React from "react";
import { detectServerVideoConfigModels, detectVideoConfigModels } from "../../../api/video";
import { BookInput } from "../../../components/ui/BookInput";
import { FormField } from "../ConfigListPanel";
import { ModelDiscoveryField } from "../config-list/ModelDiscoveryField";
import {
  VIDEO_BASE_URL_HINTS,
  VIDEO_DURATION_MODES,
  VIDEO_PROVIDERS,
  VIDEO_SIZE_OPTIONS,
  GROK_REFERENCE_MODE_OPTIONS,
  parseVideoProvider,
} from "./videoConfigUtils";
import type { VideoForm } from "./types";

interface VideoConfigEditorProps {
  form: VideoForm;
  setForm: (form: VideoForm) => void;
  configId?: number | null;
  serverScope?: boolean;
  readonly?: boolean;
  templateLocked?: boolean;
}

export const VideoConfigEditor: React.FC<VideoConfigEditorProps> = ({
  form,
  setForm,
  configId = null,
  serverScope = false,
  readonly = false,
  templateLocked = false,
}) => {
  const detectModels = serverScope ? detectServerVideoConfigModels : detectVideoConfigModels;
  const apiKeyInputDisabled = form.clear_api_key;
  const structureDisabled = readonly || templateLocked;

  const setDurationMode = (durationMode: VideoForm["duration_mode"]) => {
    if (structureDisabled) return;
    setForm({
      ...form,
      duration_mode: durationMode,
      supported_durations_text:
        durationMode === "fixed" && !form.supported_durations_text.trim()
          ? String(form.duration_secs)
          : form.supported_durations_text,
    });
  };

  return (
    <>
      <FormField label="配置名称" required>
        <BookInput
          value={form.name}
          onChange={(event) => setForm({ ...form, name: event.target.value })}
          placeholder="例如：Runway / 可灵 / 内网视频模型"
          disabled={structureDisabled}
        />
      </FormField>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <FormField label="Provider">
          <select
            value={form.provider}
            onChange={(event) =>
              setForm({ ...form, provider: parseVideoProvider(event.target.value) })
            }
            className="pyqt-input w-full"
            disabled={structureDisabled}
          >
            {VIDEO_PROVIDERS.map((provider) => (
              <option key={provider.value} value={provider.value}>
                {provider.label}
              </option>
            ))}
          </select>
        </FormField>
        <FormField label="模型" required>
          <ModelDiscoveryField
            value={form.model}
            onChange={(model) => setForm({ ...form, model })}
            placeholder="例如：video-01 / kling-v1 / runway-gen3"
            disabled={readonly}
            inputDisabled={structureDisabled}
            discoveryKey={[
              form.provider,
              form.api_base_url.trim(),
              serverScope ? "server" : "local",
              configId ?? "new",
              form.clear_api_key ? "cleared-key" : form.api_key.trim() ? "form-key" : "saved-key",
            ].join("|")}
            detectModels={() =>
              detectModels({
                provider: form.provider,
                base_url: form.api_base_url,
                api_key_encrypted: form.clear_api_key
                  ? null
                  : form.api_key.trim() || undefined,
                config_id: configId,
                use_saved_api_key: Boolean(
                  !readonly &&
                    configId != null &&
                    configId > 0 &&
                    form.has_api_key &&
                    !form.clear_api_key &&
                    !form.api_key.trim(),
                ),
              })
            }
          />
        </FormField>
      </div>

      <FormField label="API Base URL" hint={VIDEO_BASE_URL_HINTS[form.provider]}>
        <BookInput
          value={form.api_base_url}
          onChange={(event) => setForm({ ...form, api_base_url: event.target.value })}
          placeholder="https://api.example.com/v1"
          disabled={structureDisabled}
        />
      </FormField>

      <FormField
        label="API Key"
        hint={
          templateLocked
            ? "只保存为你的个人密钥；系统默认模板不会共享管理员 API Key"
            : "存储于本地数据库，不会上传到任何外部服务"
        }
      >
        <BookInput
          type="password"
          value={form.api_key}
          onChange={(event) =>
            setForm({ ...form, api_key: event.target.value, clear_api_key: false })
          }
          autoComplete="off"
          disabled={apiKeyInputDisabled || readonly}
        />
        {form.has_api_key ? (
          <label className="mt-2 flex items-center gap-2 text-xs text-book-text-muted">
            <input
              type="checkbox"
              checked={form.clear_api_key}
              disabled={readonly}
              onChange={(event) =>
                setForm({
                  ...form,
                  api_key: event.target.checked ? "" : form.api_key,
                  clear_api_key: event.target.checked,
                })
              }
            />
            清空已保存 API Key
          </label>
        ) : null}
      </FormField>

      {form.provider === "cliproxy-openai-video" ? (
        <FormField
          label="Grok 输入模式"
          hint={
            GROK_REFERENCE_MODE_OPTIONS.find(
              (option) => option.value === form.grok_reference_mode,
            )?.hint
          }
        >
          <select
            value={form.grok_reference_mode}
            onChange={(event) =>
              setForm({
                ...form,
                grok_reference_mode: event.target.value === "first_frame" ? "first_frame" : "reference",
              })
            }
            className="pyqt-input w-full"
            disabled={structureDisabled}
          >
            {GROK_REFERENCE_MODE_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </FormField>
      ) : null}

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-[minmax(0,1fr)_minmax(210px,1.2fr)_120px_100px_100px]">
        <FormField label="尺寸">
          <select
            value={form.size}
            onChange={(event) => setForm({ ...form, size: event.target.value })}
            className="pyqt-input w-full"
            disabled={structureDisabled}
          >
            {VIDEO_SIZE_OPTIONS.map((size) => (
              <option key={size} value={size}>
                {size}
              </option>
            ))}
          </select>
        </FormField>
        <FormField label="时长模式">
          <div className="grid grid-cols-2 overflow-hidden rounded border border-book-border bg-book-bg-paper">
            {VIDEO_DURATION_MODES.map((mode) => {
              const active = form.duration_mode === mode.value;
              return (
                <button
                  key={mode.value}
                  type="button"
                  onClick={() => setDurationMode(mode.value)}
                  disabled={structureDisabled}
                  className={`h-9 px-2 text-xs font-semibold transition-colors ${
                    active
                      ? "bg-book-primary/10 text-book-primary"
                      : "text-book-text-sub hover:bg-book-bg-tertiary"
                  }`}
                >
                  {mode.label}
                </button>
              );
            })}
          </div>
        </FormField>
        <FormField label="默认时长">
          <BookInput
            type="number"
            min={1}
            max={60}
            step={1}
            value={String(form.duration_secs)}
            onChange={(event) =>
              setForm({ ...form, duration_secs: Number(event.target.value) })
            }
            disabled={structureDisabled}
          />
        </FormField>
        <FormField label="FPS">
          <BookInput
            type="number"
            min={1}
            max={120}
            step={1}
            value={String(form.fps)}
            onChange={(event) => setForm({ ...form, fps: Number(event.target.value) })}
            disabled={structureDisabled}
          />
        </FormField>
        <FormField label="并发">
          <BookInput
            type="number"
            min={1}
            max={8}
            step={1}
            value={String(form.max_concurrent)}
            onChange={(event) =>
              setForm({ ...form, max_concurrent: Number(event.target.value) })
            }
            disabled={structureDisabled}
          />
        </FormField>
      </div>

      {form.duration_mode === "range" ? (
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <FormField label="最小时长">
            <BookInput
              type="number"
              min={1}
              max={60}
              step={1}
              value={String(form.min_duration_secs)}
              onChange={(event) =>
                setForm({ ...form, min_duration_secs: Number(event.target.value) })
              }
              disabled={structureDisabled}
            />
          </FormField>
          <FormField label="最大时长">
            <BookInput
              type="number"
              min={1}
              max={60}
              step={1}
              value={String(form.max_duration_secs)}
              onChange={(event) =>
                setForm({ ...form, max_duration_secs: Number(event.target.value) })
              }
              disabled={structureDisabled}
            />
          </FormField>
        </div>
      ) : (
        <FormField
          label="固定时长档位"
          hint="逗号或空格分隔秒数，例如 5,10。固定模式下只能从这些秒数中生成。"
        >
          <BookInput
            value={form.supported_durations_text}
            onChange={(event) =>
              setForm({ ...form, supported_durations_text: event.target.value })
            }
            placeholder="例如：5, 10"
            disabled={structureDisabled}
          />
        </FormField>
      )}
    </>
  );
};
