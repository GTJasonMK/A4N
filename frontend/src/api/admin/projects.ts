import { apiInvoke } from "../index";
import type { AdminProject } from "./types";
import { type ApiId as Id, idNum } from "../shared";

export async function listAllProjects(callerUserId: Id): Promise<AdminProject[]> {
  return apiInvoke<AdminProject[]>("list_all_projects", {
    callerUserId: idNum(callerUserId),
  });
}
