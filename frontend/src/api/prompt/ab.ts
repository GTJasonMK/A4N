import { apiInvoke } from "../index";
import type { AbRenderResult, RenderInput } from "../generated/host";

export async function promptRenderAb(input: RenderInput): Promise<AbRenderResult> {
  return apiInvoke<AbRenderResult>("prompt_render_ab", { input });
}
