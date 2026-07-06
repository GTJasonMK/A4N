import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法渲染提示词");
  return id;
};

export interface PromptRenderAbResult {
  version_a: string;
  version_b: string;
}

export async function promptRenderAb(input: {
  template_name: string;
  variables?: Record<string, unknown> | null;
  project_id?: number | null;
}): Promise<PromptRenderAbResult> {
  return apiInvoke<PromptRenderAbResult>("prompt_render_ab", { userId: currentUserId(), input });
}
