import { apiInvoke } from "./index";
import type * as Host from "./generated/host";
import type { ModelRequestPreview } from "./requestPreview";

export type {
  AdaptationPlanV1, AdaptationUnit, AdaptationBeat, AdaptationOrigin, AdaptationTarget,
  AuthoringDocumentRecord, AuthoringDocumentView, AuthoringDocumentKind, AuthoringChapter,
  MediaAuthoringWorkspace, GalgameBranchDesign,
} from "./generated/host";

export const getMediaAuthoringWorkspace = (input: Host.MediaAuthoringInput) =>
  apiInvoke<Host.MediaAuthoringWorkspace>("get_media_authoring_workspace", { input });
export const saveMediaAuthoringDocument = (input: Host.SaveAuthoringDocumentInput) =>
  apiInvoke<Host.AuthoringDocumentRecord>("save_media_authoring_document", { input });
export const adoptMediaAuthoringDocument = (input: Host.AdoptAuthoringDocumentInput) =>
  apiInvoke<Host.AuthoringDocumentRecord>("adopt_media_authoring_document", { input });
export const generateMediaAdaptationPlan = (input: Host.GenerateAdaptationPlanInput) =>
  apiInvoke<Host.AuthoringDocumentRecord>("generate_media_adaptation_plan", { input });
export const previewMediaAdaptationPlanRequest = (input: Host.GenerateAdaptationPlanInput) =>
  apiInvoke<ModelRequestPreview>("preview_media_adaptation_plan_request", { input });
