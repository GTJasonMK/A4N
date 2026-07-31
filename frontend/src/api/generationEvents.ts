import Ajv2020, { type ErrorObject, type ValidateFunction } from "ajv/dist/2020.js";
import generationSchema from "./generated/generation-contract.schema.json";
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

export type {
  GenerationTaskEvent,
  GenerationTaskHandle,
  GenerationTaskHistoryPage,
  GenerationTaskSnapshot,
  GenerationTaskSummary,
  HostErrorResponse,
} from "./generated/generationContracts";

const EVENT_MAX_BYTES = 64 * 1024;
const SNAPSHOT_MAX_BYTES = 4 * 1024 * 1024;
const schemaId = "a4n-generation-contract";
const ajv = new Ajv2020({
  allErrors: true,
  strict: true,
  formats: { int32: true, int64: true, uint16: true, uint32: true, uint64: true },
});

ajv.addSchema(generationSchema, schemaId);

const eventValidator = requiredValidator<GenerationTaskEvent>("GenerationTaskEvent");
const snapshotValidator = requiredValidator<GenerationTaskSnapshot>(
  "GenerationTaskSnapshot",
);
const handleValidator = requiredValidator<GenerationTaskHandle>("GenerationTaskHandle");
const subscriptionHandleValidator = requiredValidator<GenerationSubscriptionHandle>(
  "GenerationSubscriptionHandle",
);
const summaryValidator = requiredValidator<GenerationTaskSummary>("GenerationTaskSummary");
const historyValidator = requiredValidator<GenerationTaskHistoryPage>(
  "GenerationTaskHistoryPage",
);
const hostErrorValidator = requiredValidator<HostErrorResponse>("HostErrorResponse");

export function parseGenerationTaskEvent(value: unknown): GenerationTaskEvent {
  assertJsonSize(value, EVENT_MAX_BYTES, "generation event");
  if (!eventValidator(value)) {
    throw new Error(`generation event schema invalid: ${formatErrors(eventValidator.errors)}`);
  }
  return value;
}

export function parseGenerationTaskSnapshot(value: unknown): GenerationTaskSnapshot {
  assertJsonSize(value, SNAPSHOT_MAX_BYTES, "generation snapshot");
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

function requiredValidator<T>(name: string): ValidateFunction<T> {
  const validator = ajv.getSchema<T>(`${schemaId}#/$defs/${name}`);
  if (!validator) throw new Error(`${name} schema is missing`);
  return validator;
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
