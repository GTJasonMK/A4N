import { apiInvoke } from "../index";
import type { ImportReviewListItem, ImportReviewSummary } from "../generated/host";
import { type ApiId as Id, idNum } from "../shared";

export async function listImportQueue(projectId: Id): Promise<ImportReviewSummary[]> {
  return apiInvoke<ImportReviewSummary[]>("list_import_queue", {
    input: { project_id: idNum(projectId) },
  });
}

export async function listImportQueueIndex(projectId: Id): Promise<ImportReviewListItem[]> {
  return apiInvoke<ImportReviewListItem[]>("list_import_queue_index", {
    input: { project_id: idNum(projectId) },
  });
}

export async function getImportReview(reviewId: Id): Promise<ImportReviewSummary> {
  return apiInvoke<ImportReviewSummary>("get_import_review", {
    input: { review_id: idNum(reviewId) },
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
): Promise<ImportReviewSummary> {
  return apiInvoke<ImportReviewSummary>("accept_import_review", {
    input: {
      review_id: idNum(reviewId),
      ...(analysisResult === undefined ? {} : { analysis_result: analysisResult }),
    },
  });
}

export async function rejectImportReview(reviewId: Id): Promise<ImportReviewSummary> {
  return apiInvoke<ImportReviewSummary>("reject_import_review", {
    input: { review_id: idNum(reviewId) },
  });
}

export async function batchAcceptReviews(reviewIds: number[]): Promise<ImportReviewSummary[]> {
  return apiInvoke<ImportReviewSummary[]>("batch_accept_reviews", {
    input: { review_ids: reviewIds },
  });
}
