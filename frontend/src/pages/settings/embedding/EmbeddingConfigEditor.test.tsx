import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { EmbeddingConfigEditor } from "./EmbeddingConfigEditor";
import type { EmbeddingForm } from "./types";
import { I18nProvider } from "../../../i18n";
import type { LocalEmbeddingModel } from "../../../api/config";

describe("EmbeddingConfigEditor", () => {
  it("keeps the saved fastembed model visible when the model catalog is unavailable", () => {
    const form: EmbeddingForm = {
      name: "env15",
      backend: "fastembed",
      model: "BGEBaseENV15",
      api_base_url: "",
      api_key: "",
      has_api_key: false,
      clear_api_key: false,
      dimension: 768,
      batch_size: 32,
      local_model_path: "",
    };
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <EmbeddingConfigEditor
        form={form}
        providers={[
          { id: "fastembed", display_name: "FastEmbed 本地模型" },
        ]}
        localModels={[]}
        busyModel={null}
        setForm={() => undefined}
        pickFastembedModel={() => null}
        resolveLocalModel={() => null}
        resolveDownloadInfo={() => null}
        modelHostLabel="远程服务器"
        onDownloadModel={() => undefined}
        onDeleteModel={() => undefined}
        />
      </I18nProvider>,
    );

    expect(html).toContain('<option value="BGEBaseENV15" selected="">BGEBaseENV15</option>');
  });

  it("keeps model options compact when download metadata is available", () => {
    const model: LocalEmbeddingModel = {
      name: "BGESmallZHV15",
      model_code: "BAAI/bge-small-zh-v1.5",
      description: "v1.5 release of the small Chinese model",
      dimension: 512,
      downloaded: true,
      size_bytes: 95_000_000,
    };
    const form: EmbeddingForm = {
      name: "fastembed",
      backend: "fastembed",
      model: model.name,
      api_base_url: "",
      api_key: "",
      has_api_key: false,
      clear_api_key: false,
      dimension: model.dimension,
      batch_size: 32,
      local_model_path: "",
    };
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <EmbeddingConfigEditor
          form={form}
          providers={[{ id: "fastembed", display_name: "FastEmbed 本地模型" }]}
          localModels={[model]}
          busyModel={null}
          setForm={() => undefined}
          pickFastembedModel={() => model}
          resolveLocalModel={() => model}
          resolveDownloadInfo={() => null}
          modelHostLabel="本机"
          onDownloadModel={() => undefined}
          onDeleteModel={() => undefined}
        />
      </I18nProvider>,
    );

    expect(html).toContain(
      '<option value="BGESmallZHV15" selected="">BGESmallZHV15 · 512 维 · 已下载</option>',
    );
  });
});
