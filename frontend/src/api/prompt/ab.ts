import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { AbRenderResult, RenderInput } from "../generated/host";

export async function promptRenderAb(input: RenderInput): Promise<AbRenderResult> {
  return apiInvoke<AbRenderResult>("prompt_render_ab", { userId: currentUserId(), input });
}
