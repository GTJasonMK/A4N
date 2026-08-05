import React from "react";
import { BookInput, BookTextarea } from "../../../components/ui/BookInput";
import { detectLlmConfigModels, detectServerLlmConfigModels } from "../../../api/config";
import { FormField } from "../ConfigListPanel";
import { ClearSavedApiKeySwitch } from "../config-list/ClearSavedApiKeySwitch";
import { ModelDiscoveryField } from "../config-list/ModelDiscoveryField";
import type { LlmForm } from "./types";
import type { LlmProviderDescriptor } from "../../../api/generated/config";

interface LlmConfigEditorProps {
  form: LlmForm;
  setForm: (form: LlmForm) => void;
  configId?: number | null;
  serverScope?: boolean;
  readonly?: boolean;
  templateLocked?: boolean;
  providers: LlmProviderDescriptor[];
}

export const LlmConfigEditor: React.FC<LlmConfigEditorProps> = ({
  form,
  setForm,
  configId = null,
  serverScope = false,
  readonly = false,
  templateLocked = false,
  providers,
}) => {
  const apiKeyInputDisabled = form.clear_api_key;
  const structureDisabled = readonly || templateLocked;
  const detectModels = serverScope ? detectServerLlmConfigModels : detectLlmConfigModels;
  const selectedProvider = providers.find((provider) => provider.id === form.provider);

  return (
    <>
      <FormField label="配置名称" required>
        <BookInput
          value={form.name}
          onChange={(event) => setForm({ ...form, name: event.target.value })}
          placeholder="例如：日常 / 创作主力 / 审稿"
          disabled={structureDisabled}
        />
      </FormField>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <FormField label="服务商">
          <select
            value={form.provider}
            onChange={(event) => {
              const provider = event.target.value;
              const currentDefault = providers.find(
                (item) => item.id === form.provider,
              )?.default_base_url;
              const nextDefault = providers.find(
                (item) => item.id === provider,
              )?.default_base_url;
              const baseUrl = form.api_base_url.trim();
              setForm({
                ...form,
                provider,
                api_base_url:
                  !baseUrl || baseUrl === currentDefault
                    ? nextDefault ?? ""
                    : form.api_base_url,
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
            placeholder="例如：deepseek-v4-flash / gpt-4o-mini / claude-3-5-sonnet"
            disabled={readonly || !selectedProvider}
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

      <FormField label="接口地址" required>
        <BookInput
          value={form.api_base_url}
          onChange={(event) => setForm({ ...form, api_base_url: event.target.value })}
          placeholder={selectedProvider?.default_base_url ?? "https://api.example.com/v1"}
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
            setForm({ ...form, api_key: event.target.value, clear_api_key: false })
          }
          placeholder="sk-..."
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

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <FormField
          label="并发上限"
          hint="该配置同时允许的最大 LLM 请求数；超过则进入任务队列等待"
        >
          <BookInput
            type="number"
            min={1}
            max={32}
            step={1}
            value={String(form.max_concurrent)}
            onChange={(event) =>
              setForm({ ...form, max_concurrent: Number(event.target.value) })
            }
            disabled={structureDisabled}
          />
        </FormField>

        <FormField label="超时时间（秒）" hint="单次 LLM 请求等待响应的最长秒数；视频、漫画等长结构化任务建议不少于 300 秒">
          <BookInput
            type="number"
            min={1}
            max={1800}
            step={1}
            value={String(form.timeout_secs)}
            onChange={(event) => setForm({ ...form, timeout_secs: Number(event.target.value) })}
            disabled={structureDisabled}
          />
        </FormField>
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <FormField
          label="模型上下文窗口"
          hint="模型一次请求可容纳的输入与输出总 token 数；DeepSeek 主力模型通常按 128000 配置"
        >
          <BookInput
            type="number"
            min={1024}
            max={1048576}
            step={1024}
            value={String(form.max_context_tokens)}
            onChange={(event) =>
              setForm({ ...form, max_context_tokens: Number(event.target.value) })
            }
            disabled={structureDisabled}
          />
        </FormField>
        <FormField
          label="模型能力：最大输出"
          hint="模型单次可生成的硬上限；场景请求输出预算必须小于或等于这个值"
        >
          <BookInput
            type="number"
            min={1}
            max={262144}
            step={256}
            value={String(form.max_output_tokens)}
            onChange={(event) =>
              setForm({ ...form, max_output_tokens: Number(event.target.value) })
            }
            disabled={structureDisabled}
          />
        </FormField>
      </div>

      <FormField
        label="高级请求参数"
        hint="仅填写服务商扩展字段；模型、消息、流式开关和输出上限等系统字段会被拒绝"
      >
        <BookTextarea
          rows={6}
          value={form.extra_body_text}
          onChange={(event) =>
            setForm({ ...form, extra_body_text: event.target.value })
          }
          placeholder={'例如：{\n  "reasoning_effort": "low"\n}'}
          spellCheck={false}
          disabled={structureDisabled}
        />
      </FormField>
    </>
  );
};
