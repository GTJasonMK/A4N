import type { UserSummary } from "../generated/auth";
import type {
  CreateInviteCodeInput,
  CreateUserInput,
  InviteCodeSummary,
} from "../generated/host";
import { apiInvoke } from "../index";
import { type ApiId as Id, idNum } from "../shared";

export async function listAllUsers(callerUserId: Id): Promise<UserSummary[]> {
  return apiInvoke<UserSummary[]>("list_all_users", {
    callerUserId: idNum(callerUserId),
  });
}

export async function createAdminUser(
  callerUserId: Id,
  input: CreateUserInput,
): Promise<UserSummary> {
  return apiInvoke<UserSummary>("create_user", {
    callerUserId: idNum(callerUserId),
    input,
  });
}

export async function listInviteCodes(callerUserId: Id): Promise<InviteCodeSummary[]> {
  return apiInvoke<InviteCodeSummary[]>("list_invite_codes", {
    callerUserId: idNum(callerUserId),
  });
}

export async function createInviteCode(
  callerUserId: Id,
  input: CreateInviteCodeInput,
): Promise<InviteCodeSummary> {
  return apiInvoke<InviteCodeSummary>("create_invite_code", {
    callerUserId: idNum(callerUserId),
    input,
  });
}

export async function updateUserRole(
  callerUserId: Id,
  userId: Id,
  role: string,
): Promise<UserSummary> {
  return apiInvoke<UserSummary>("update_user_role", {
    callerUserId: idNum(callerUserId),
    userId: idNum(userId),
    input: { role },
  });
}

export async function disableUser(callerUserId: Id, userId: Id): Promise<void> {
  return apiInvoke<void>("disable_user", {
    callerUserId: idNum(callerUserId),
    userId: idNum(userId),
  });
}
