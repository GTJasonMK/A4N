import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { RenderInput } from "../generated/host";

export async function promptRender(input: RenderInput): Promise<string> {
  return apiInvoke<string>("prompt_render", { userId: currentUserId(), input });
}
