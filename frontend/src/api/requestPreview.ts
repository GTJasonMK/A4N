export type {
  ModelRequestPreview,
  ModelRequestPreviewStep,
} from "./generated/host";

export interface ModelRequestPreviewMessage {
  role?: string;
  content?: string;
  [key: string]: unknown;
}
