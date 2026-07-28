import type {
  ModelRequestPreview as GeneratedModelRequestPreview,
  ModelRequestPreviewStep as GeneratedModelRequestPreviewStep,
} from "./generated/host";

export interface ModelRequestPreviewMessage {
  role?: string;
  content?: string;
  [key: string]: unknown;
}

export type ModelRequestPreviewStep = Omit<GeneratedModelRequestPreviewStep, "state"> & {
  state: "ready" | "preview_placeholder" | "blocked" | "reused" | string;
};

export type ModelRequestPreview = Omit<GeneratedModelRequestPreview, "steps"> & {
  steps: ModelRequestPreviewStep[];
};
