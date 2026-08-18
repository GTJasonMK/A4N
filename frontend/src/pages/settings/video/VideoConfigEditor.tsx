import React from "react";
import { detectVideoConfigModels } from "../../../api/video";
import { BookInput } from "../../../components/ui/BookInput";
import { FormField } from "../ConfigListPanel";
import { ClearSavedApiKeySwitch } from "../config-list/ClearSavedApiKeySwitch";
import { ModelDiscoveryField } from "../config-list/ModelDiscoveryField";
import { VIDEO_DURATION_MODES, VIDEO_SIZE_OPTIONS } from "./videoConfigUtils";
import type { VideoProviderDescriptor } from "../../../api/generated/config";
import { buildModelDiscoveryField } from "../../../api/modelDiscovery";
import type { ModelConfigScope } from "../../../api/modelDiscovery";
import type { VideoForm } from "./types";

interface VideoConfigEditorProps {
  form: VideoForm;
  setForm: (form: VideoForm) => void;
  configId?: number | null;
  scope: ModelConfigScope;
  readonly?: boolean;
  templateLocked?: boolean;
  providers: VideoProviderDescriptor[];
}

export const VideoConfigEditor: React.FC<VideoConfigEditorProps> = ({
  form,
  setForm,
  configId = null,
  scope,
  readonly = false,
  templateLocked = false,
  providers,
}) => {
  const apiKeyInputDisabled = form.clear_api_key;
  const structureDisabled = readonly || templateLocked;
  const selectedProvider = providers.find(
    (provider) => provider.id === form.provider,
  );
  const providerMinDuration =
    selectedProvider?.min_duration_secs ?? form.min_duration_secs;
  const providerMaxDuration =
    selectedProvider?.max_duration_secs ?? form.max_duration_secs;
  const modelDiscovery = buildModelDiscoveryField({
    provider: form.provider,
    baseUrl: form.api_base_url,
    apiKey: form.api_key,
    clearApiKey: form.clear_api_key,
    configId,
    hasApiKey: form.has_api_key,
    readonly,
    scope,
  });

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
        <FormField label="视频服务商">
          <select
            value={form.provider}
            onChange={(event) => {
              const provider = providers.find(
                (item) => item.id === event.target.value,
              );
              if (!provider)
                throw new Error(`video provider 未注册：${event.target.value}`);
              setForm({
                ...form,
                provider: provider.id,
                duration_secs: Math.min(
                  provider.max_duration_secs,
                  Math.max(provider.min_duration_secs, form.duration_secs),
                ),
                min_duration_secs: provider.min_duration_secs,
                max_duration_secs: provider.max_duration_secs,
              });
            }}
            className="pyqt-input w-full"
            disabled={structureDisabled}
          >
            {providers.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.display_name}
              </option>
            ))}
          </select>
        </FormField>
        <FormField label="模型" required>
          <ModelDiscoveryField
            value={form.model}
            onChange={(model) => setForm({ ...form, model })}
            placeholder="例如：video-01 / kling-v1 / runway-gen3"
            disabled={readonly || !selectedProvider}
            inputDisabled={structureDisabled}
            discoveryKey={modelDiscovery.discoveryKey}
            detectModels={() => detectVideoConfigModels(modelDiscovery.input, scope)}
          />
        </FormField>
      </div>

      <FormField label="接口地址">
        <BookInput
          value={form.api_base_url}
          onChange={(event) =>
            setForm({ ...form, api_base_url: event.target.value })
          }
          placeholder="https://api.example.com/v1"
          disabled={structureDisabled}
        />
      </FormField>

      <FormField
        label="访问密钥"
        hint={
          templateLocked
            ? "只保存为你的个人密钥；系统默认模板不会共享管理员密钥"
            : "存储于本地数据库，不会上传到任何外部服务"
        }
      >
        <BookInput
          type="password"
          value={form.api_key}
          onChange={(event) =>
            setForm({
              ...form,
              api_key: event.target.value,
              clear_api_key: false,
            })
          }
          autoComplete="off"
          disabled={apiKeyInputDisabled || readonly}
        />
      </FormField>
      {form.has_api_key ? (
        <ClearSavedApiKeySwitch
          checked={form.clear_api_key}
          disabled={readonly}
          onChange={(checked) =>
            setForm({
              ...form,
              api_key: checked ? "" : form.api_key,
              clear_api_key: checked,
            })
          }
        />
      ) : null}

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
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
                  aria-pressed={active}
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
            min={providerMinDuration}
            max={providerMaxDuration}
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
            onChange={(event) =>
              setForm({ ...form, fps: Number(event.target.value) })
            }
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
              min={providerMinDuration}
              max={providerMaxDuration}
              step={1}
              value={String(form.min_duration_secs)}
              onChange={(event) =>
                setForm({
                  ...form,
                  min_duration_secs: Number(event.target.value),
                })
              }
              disabled={structureDisabled}
            />
          </FormField>
          <FormField label="最大时长">
            <BookInput
              type="number"
              min={providerMinDuration}
              max={providerMaxDuration}
              step={1}
              value={String(form.max_duration_secs)}
              onChange={(event) =>
                setForm({
                  ...form,
                  max_duration_secs: Number(event.target.value),
                })
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
