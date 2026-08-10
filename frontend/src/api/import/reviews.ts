import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { ImportReview, ImportReviewStatus, ModifyImportReviewInput } from "./types";
import { type ApiId as Id, idNum } from "../shared";

export async function listImportQueue(projectId: Id): Promise<ImportReview[]> {
  return apiInvoke<ImportReview[]>("list_import_queue", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function updateImportStatus(
  reviewId: Id,
  status: ImportReviewStatus,
): Promise<ImportReview> {
  return apiInvoke<ImportReview>("update_import_status", {
    userId: currentUserId(),
    reviewId: idNum(reviewId),
    input: { status },
  });
}

export async function deleteImportReview(reviewId: Id): Promise<void> {
  return apiInvoke<void>("delete_import_review", {
    userId: currentUserId(),
    reviewId: idNum(reviewId),
  });
}

export async function getImportReview(reviewId: Id): Promise<ImportReview> {
  return apiInvoke<ImportReview>("get_import_review", {
    userId: currentUserId(),
    reviewId: idNum(reviewId),
  });
}

export async function acceptImportReview(reviewId: Id): Promise<ImportReview> {
  return apiInvoke<ImportReview>("accept_import_review", {
    userId: currentUserId(),
    reviewId: idNum(reviewId),
  });
}

export async function modifyImportReview(
  reviewId: Id,
  input: ModifyImportReviewInput,
): Promise<ImportReview> {
  return apiInvoke<ImportReview>("modify_import_review", {
    userId: currentUserId(),
    reviewId: idNum(reviewId),
    input,
  });
}

export async function rejectImportReview(reviewId: Id): Promise<ImportReview> {
  return apiInvoke<ImportReview>("reject_import_review", {
    userId: currentUserId(),
    reviewId: idNum(reviewId),
  });
}

export async function batchAcceptReviews(reviewIds: number[]): Promise<ImportReview[]> {
  return apiInvoke<ImportReview[]>("batch_accept_reviews", {
    userId: currentUserId(),
    input: { review_ids: reviewIds },
  });
}
