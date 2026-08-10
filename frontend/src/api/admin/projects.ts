import { apiInvoke } from "../index";
import type { AdminProjectSummary } from "../generated/host";
import { type ApiId as Id, idNum } from "../shared";

export async function listAllProjects(callerUserId: Id): Promise<AdminProjectSummary[]> {
  return apiInvoke<AdminProjectSummary[]>("list_all_projects", {
    callerUserId: idNum(callerUserId),
  });
}
