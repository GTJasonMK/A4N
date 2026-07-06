import type { LlmMaxTokensConfig } from "../../../api/config";

export const MAX_TOKEN_LIMIT = 262_144;

type TokenFieldKey = keyof LlmMaxTokensConfig;

export const TOKEN_GROUPS: Array<{
  title: string;
  fields: Array<{ key: TokenFieldKey; label: string }>;
}> = [
  {
    title: "基础场景",
    fields: [
      { key: "connection_test", label: "连接测试" },
      { key: "short", label: "短输出" },
      { key: "standard", label: "常规输出" },
      { key: "json_standard", label: "结构化 JSON" },
      { key: "long", label: "长输出" },
      { key: "extra_long", label: "超长输出" },
    ],
  },
  {
    title: "业务生成",
    fields: [
      { key: "chapter_generation", label: "章节正文" },
      { key: "chapter_analysis", label: "章节分析" },
      { key: "outline", label: "分部/章节大纲" },
      { key: "blueprint", label: "故事蓝图" },
      { key: "character_discovery", label: "角色发现" },
      { key: "character_sync", label: "角色同步" },
      { key: "media_prompt", label: "图片/漫画/视频提示词" },
    ],
  },
];

const TOKEN_FIELD_KEYS = TOKEN_GROUPS.flatMap((group) =>
  group.fields.map((field) => field.key),
);

export function validateMaxTokensConfig(
  value: LlmMaxTokensConfig,
  modelMaxOutputTokens?: number | null,
): string | null {
  for (const key of TOKEN_FIELD_KEYS) {
    const numberValue = value[key];
    if (!Number.isInteger(numberValue) || numberValue <= 0) {
      return `${tokenLabel(key)} 必须为正整数`;
    }
    if (numberValue > MAX_TOKEN_LIMIT) {
      return `${tokenLabel(key)} 不能超过 ${MAX_TOKEN_LIMIT}`;
    }
    if (
      modelMaxOutputTokens != null &&
      Number.isInteger(modelMaxOutputTokens) &&
      modelMaxOutputTokens > 0 &&
      numberValue > modelMaxOutputTokens
    ) {
      return `${tokenLabel(key)} 不能超过当前默认模型最大输出 ${modelMaxOutputTokens}`;
    }
  }
  return null;
}

function tokenLabel(key: TokenFieldKey): string {
  for (const group of TOKEN_GROUPS) {
    const field = group.fields.find((item) => item.key === key);
    if (field) return field.label;
  }
  return key;
}

export function maxTokensSummary(
  form: LlmMaxTokensConfig | null,
  loading: boolean,
  modelMaxOutputTokens?: number | null,
): string {
  if (loading) return "正在加载";
  if (!form) return "未加载";
  const modelCap = modelMaxOutputTokens
    ? ` · 默认模型最大输出 ${modelMaxOutputTokens}`
    : "";
  return `章节 ${form.chapter_generation} · 大纲 ${form.outline} · 角色发现 ${form.character_discovery} · 同步 ${form.character_sync}${modelCap}`;
}
