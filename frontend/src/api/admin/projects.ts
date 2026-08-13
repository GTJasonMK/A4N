import { apiInvoke } from "../index";
import type { AdminProjectSummary } from "../generated/host";

export async function listAllProjects(): Promise<AdminProjectSummary[]> {
  return apiInvoke<AdminProjectSummary[]>("list_all_projects");
}
