import { apiInvoke } from "../index";
import type {
  AdminInviteCode,
  AdminUser,
  CreateAdminUserInput,
  CreateInviteCodeInput,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";

export async function listAllUsers(callerUserId: Id): Promise<AdminUser[]> {
  return apiInvoke<AdminUser[]>("list_all_users", {
    callerUserId: idNum(callerUserId),
  });
}

export async function createAdminUser(
  callerUserId: Id,
  input: CreateAdminUserInput,
): Promise<AdminUser> {
  return apiInvoke<AdminUser>("create_user", {
    callerUserId: idNum(callerUserId),
    input,
  });
}

export async function listInviteCodes(callerUserId: Id): Promise<AdminInviteCode[]> {
  return apiInvoke<AdminInviteCode[]>("list_invite_codes", {
    callerUserId: idNum(callerUserId),
  });
}

export async function createInviteCode(
  callerUserId: Id,
  input: CreateInviteCodeInput,
): Promise<AdminInviteCode> {
  return apiInvoke<AdminInviteCode>("create_invite_code", {
    callerUserId: idNum(callerUserId),
    input,
  });
}

export async function updateUserRole(
  callerUserId: Id,
  userId: Id,
  role: string,
): Promise<AdminUser> {
  return apiInvoke<AdminUser>("update_user_role", {
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
