import React from "react";
import { detectImageConfigModels, detectServerImageConfigModels } from "../../../api/image";
import { BookInput } from "../../../components/ui/BookInput";
import { FormField } from "../ConfigListPanel";
import { ClearSavedApiKeySwitch } from "../config-list/ClearSavedApiKeySwitch";
import { ModelDiscoveryField } from "../config-list/ModelDiscoveryField";
import {
  IMAGE_SIZE_OPTIONS,
} from "./imageConfigUtils";
import type { ImageProviderDescriptor } from "../../../api/generated/config";
import type { ImageForm } from "./types";

interface ImageConfigEditorProps {
  form: ImageForm;
  setForm: (form: ImageForm) => void;
  configId?: number | null;
  serverScope?: boolean;
  readonly?: boolean;
  templateLocked?: boolean;
  providers: ImageProviderDescriptor[];
}

const IMAGE_SIZE_DATALIST_ID = "image-config-size-options";

export const ImageConfigEditor: React.FC<ImageConfigEditorProps> = ({
  form,
  setForm,
  configId = null,
  serverScope = false,
  readonly = false,
  templateLocked = false,
  providers,
}) => {
  const detectModels = serverScope ? detectServerImageConfigModels : detectImageConfigModels;
  const apiKeyInputDisabled = form.clear_api_key;
  const structureDisabled = readonly || templateLocked;
  const selectedProvider = providers.find((provider) => provider.id === form.provider);

  return (
    <>
      <FormField label="配置名称" required>
        <BookInput
          value={form.name}
          onChange={(event) => setForm({ ...form, name: event.target.value })}
          placeholder="例如：DALL·E 高清 / SD 写实风"
          disabled={structureDisabled}
        />
      </FormField>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <FormField label="图片服务商">
          <select
            value={form.provider}
            onChange={(event) =>
              setForm({ ...form, provider: event.target.value })
            }
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
            placeholder="例如：gpt-image-2 / dall-e-3 / sd_xl_base_1.0"
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
          placeholder="例如：https://api.openai.com/v1"
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
          label="尺寸"
          hint="可手动输入宽×高；4K 如 3840×2160，实际支持范围取决于模型和图片服务商"
        >
          <BookInput
            value={form.size}
            list={IMAGE_SIZE_DATALIST_ID}
            onChange={(event) => setForm({ ...form, size: event.target.value })}
            placeholder="例如：3840x2160"
            disabled={structureDisabled}
          />
          <datalist id={IMAGE_SIZE_DATALIST_ID}>
            {IMAGE_SIZE_OPTIONS.map((size) => (
              <option key={size} value={size}>
                {size}
              </option>
            ))}
          </datalist>
        </FormField>
        <FormField label="并发上限" hint="同一配置的最大并行图片生成数">
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
    </>
  );
};
