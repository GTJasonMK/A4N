import type { ErrorObject } from "ajv";
import {
  eventValidator,
  snapshotValidator,
  handleValidator,
  subscriptionHandleValidator,
  summaryValidator,
  historyValidator,
  hostErrorValidator,
} from "./generated/generationValidators.js";
import type {
  GenerationTaskHandle,
  GenerationTaskHistoryPage,
  GenerationTaskEvent,
  GenerationSubscriptionHandle,
  GenerationTaskSnapshot,
  GenerationTaskSummary,
  HostErrorResponse,
} from "./generated/generationContracts";
import { utf8ByteLength } from "../utils/text";
import { MAX_GENERATION_EVENT_BYTES, MAX_GENERATION_SNAPSHOT_BYTES } from "./generated/host";

export type {
  GenerationTaskEvent,
  GenerationTaskHandle,
  GenerationTaskHistoryPage,
  GenerationTaskSnapshot,
  GenerationTaskSummary,
  HostErrorResponse,
} from "./generated/generationContracts";

export function parseGenerationTaskEvent(value: unknown): GenerationTaskEvent {
  assertJsonSize(value, MAX_GENERATION_EVENT_BYTES, "generation event");
  if (!eventValidator(value)) {
    throw new Error(`generation event schema invalid: ${formatErrors(eventValidator.errors)}`);
  }
  return value;
}

export function parseGenerationTaskSnapshot(value: unknown): GenerationTaskSnapshot {
  assertJsonSize(value, MAX_GENERATION_SNAPSHOT_BYTES, "generation snapshot");
  if (!snapshotValidator(value)) {
    throw new Error(
      `generation snapshot schema invalid: ${formatErrors(snapshotValidator.errors)}`,
    );
  }
  return value;
}

export function parseGenerationTaskHandle(value: unknown): GenerationTaskHandle {
  if (!handleValidator(value)) {
    throw new Error(`generation handle schema invalid: ${formatErrors(handleValidator.errors)}`);
  }
  return value;
}

export function parseGenerationSubscriptionHandle(
  value: unknown,
): GenerationSubscriptionHandle {
  if (!subscriptionHandleValidator(value)) {
    throw new Error(
      `generation subscription handle schema invalid: ${formatErrors(subscriptionHandleValidator.errors)}`,
    );
  }
  return value;
}

export function parseGenerationTaskSummary(value: unknown): GenerationTaskSummary {
  if (!summaryValidator(value)) {
    throw new Error(`generation summary schema invalid: ${formatErrors(summaryValidator.errors)}`);
  }
  return value;
}

export function parseGenerationTaskHistoryPage(value: unknown): GenerationTaskHistoryPage {
  if (!historyValidator(value)) {
    throw new Error(`generation history schema invalid: ${formatErrors(historyValidator.errors)}`);
  }
  return value;
}

export function parseHostErrorResponse(value: unknown): HostErrorResponse {
  if (!hostErrorValidator(value)) {
    throw new Error(`generation host error schema invalid: ${formatErrors(hostErrorValidator.errors)}`);
  }
  return value;
}

function assertJsonSize(value: unknown, limit: number, label: string): void {
  let json: string | undefined;
  try {
    json = JSON.stringify(value);
  } catch (error) {
    const detail = error instanceof Error ? `: ${error.message}` : "";
    throw new Error(`${label} is not JSON serializable${detail}`);
  }
  if (json === undefined) throw new Error(`${label} is not JSON serializable`);
  const bytes = utf8ByteLength(json);
  if (bytes > limit) throw new Error(`${label} exceeds ${limit} bytes`);
}

function formatErrors(errors: ErrorObject[] | null | undefined): string {
  return (errors ?? [])
    .slice(0, 4)
    .map((error) => `${error.instancePath || "/"} ${error.message ?? error.keyword}`)
    .join("; ");
}
