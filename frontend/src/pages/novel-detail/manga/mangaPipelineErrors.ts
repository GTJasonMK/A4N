const DEFAULT_LLM_AUTH_HINT =
  "漫画流水线启动阶段使用 LLM 服务，不使用生图模型；生图模型测试通过不能证明 LLM 可用。当前使用的是“系统默认 LLM 模板 + 你的个人 API Key”。请到 设置 -> LLM 配置，选中系统默认配置，重新填写 API Key，并在连接测试通过后再运行。";

export function explainMangaPipelineAiConfigError(message: string): string {
  const text = message.trim();
  if (!text) return text;

  if (text.includes("使用系统默认 LLM 需要先")) {
    return "当前默认 LLM 是系统默认模板，但还没有保存你的个人 API Key。漫画流水线启动阶段使用 LLM 服务，不使用生图模型。请到 设置 -> LLM 配置，选中系统默认配置，填写 API Key 并保存后再运行。";
  }

  if (
    text.includes("默认 LLM 配置鉴权失败") ||
    text.includes("invalid_api_key") ||
    text.includes("Invalid API key")
  ) {
    return `默认 LLM 配置鉴权失败：${DEFAULT_LLM_AUTH_HINT} 原始错误：${text}`;
  }

  if (text.includes("默认 LLM 配置对应账号余额不足或额度耗尽")) {
    return `默认 LLM 配置对应账号余额不足或额度耗尽：请充值该供应商账号，或到 设置 -> LLM 配置 切换可用默认配置。原始错误：${text}`;
  }

  return text;
}
