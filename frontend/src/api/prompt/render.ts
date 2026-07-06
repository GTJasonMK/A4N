import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法渲染提示词");
  return id;
};

export async function promptRender(input: {
  template_name: string;
  variables?: Record<string, unknown> | null;
  project_id?: number | null;
}): Promise<string> {
  return apiInvoke<string>("prompt_render", { userId: currentUserId(), input });
}
