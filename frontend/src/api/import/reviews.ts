import { apiInvoke } from "../index";
import type { ImportReview } from "./types";
import { type ApiId as Id, idNum } from "../shared";

export async function listImportQueue(projectId: Id): Promise<ImportReview[]> {
  return apiInvoke<ImportReview[]>("list_import_queue", {
    input: { project_id: idNum(projectId) },
  });
}

export async function deleteImportReview(reviewId: Id): Promise<void> {
  return apiInvoke<void>("delete_import_review", {
    input: { review_id: idNum(reviewId) },
  });
}

export async function acceptImportReview(
  reviewId: Id,
  analysisResult?: unknown,
): Promise<ImportReview> {
  return apiInvoke<ImportReview>("accept_import_review", {
    input: {
      review_id: idNum(reviewId),
      ...(analysisResult === undefined ? {} : { analysis_result: analysisResult }),
    },
  });
}

export async function rejectImportReview(reviewId: Id): Promise<ImportReview> {
  return apiInvoke<ImportReview>("reject_import_review", {
    input: { review_id: idNum(reviewId) },
  });
}

export async function batchAcceptReviews(reviewIds: number[]): Promise<ImportReview[]> {
  return apiInvoke<ImportReview[]>("batch_accept_reviews", {
    input: { review_ids: reviewIds },
  });
}
