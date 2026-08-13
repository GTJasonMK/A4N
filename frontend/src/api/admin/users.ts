import type { UserSummary } from "../generated/auth";
import type {
  CreateInviteCodeInput,
  CreateUserInput,
  InviteCodeSummary,
} from "../generated/host";
import { apiInvoke } from "../index";
import { type ApiId as Id, idNum } from "../shared";

export async function listAllUsers(): Promise<UserSummary[]> {
  return apiInvoke<UserSummary[]>("list_all_users");
}

export async function createAdminUser(
  input: CreateUserInput,
): Promise<UserSummary> {
  return apiInvoke<UserSummary>("create_user", {
    input,
  });
}

export async function listInviteCodes(): Promise<InviteCodeSummary[]> {
  return apiInvoke<InviteCodeSummary[]>("list_invite_codes");
}

export async function createInviteCode(
  input: CreateInviteCodeInput,
): Promise<InviteCodeSummary> {
  return apiInvoke<InviteCodeSummary>("create_invite_code", {
    input,
  });
}

export async function updateUserRole(
  userId: Id,
  role: string,
): Promise<UserSummary> {
  return apiInvoke<UserSummary>("update_user_role", {
    userId: idNum(userId),
    input: { role },
  });
}

export async function disableUser(userId: Id): Promise<void> {
  return apiInvoke<void>("disable_user", {
    userId: idNum(userId),
  });
}
