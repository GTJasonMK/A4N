import { apiInvoke } from "../index";
import type { RenderInput } from "../generated/host";

export async function promptRender(input: RenderInput): Promise<string> {
  return apiInvoke<string>("prompt_render", { input });
}
