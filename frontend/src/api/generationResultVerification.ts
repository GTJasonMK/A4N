import type { GenerationArtifactRef } from "./generated/generationContracts";
import type { GenerationTaskViewState } from "../utils/generationEventReducer";
import { sha256Json } from "../utils/contentHash";
import { markGenerationConsistencyFailure } from "./generationTasks";

interface GeneratedMediaRecord {
  id: number;
}

interface GeneratedMediaResultSpec<T extends GeneratedMediaRecord> {
  projectId: number;
  singleKind: string;
  batchKind: string;
  artifactKind: string;
  artifactIdPrefix: string;
  recordInScope: (record: T) => boolean;
  recordRevision?: (record: T) => Promise<string | null>;
  errorMessage: string;
}

export async function verifyGeneratedMediaResult<T extends GeneratedMediaRecord>(
  task: GenerationTaskViewState,
  records: readonly T[],
  spec: GeneratedMediaResultSpec<T>,
): Promise<[T, ...T[]]> {
  const resultRef = task.resultRef;
  const resultId = resultRef == null ? null : positiveInteger(resultRef.id);
  const recordsById = new Map(
    records.filter(spec.recordInScope).map((record) => [record.id, record]),
  );
  const artifacts = task.artifacts.filter(
    (artifact) => artifact.artifact_kind === spec.artifactKind && !artifact.temporary,
  );

  if (resultRef?.kind === spec.singleKind && resultId != null && artifacts.length === 1) {
    const record = recordsById.get(resultId);
    const artifact = artifacts[0];
    if (
      record
      && artifact.artifact_id === `${spec.artifactIdPrefix}${resultId}`
      && canonicalArtifact(artifact)
    ) {
      const revision = spec.recordRevision
        ? await spec.recordRevision(record)
        : artifact.revision;
      if (revision != null && revision === resultRef.revision) return [record];
    }
  } else if (
    resultRef?.kind === spec.batchKind
    && resultId === spec.projectId
    && artifacts.length > 0
  ) {
    const ids: number[] = [];
    for (const artifact of artifacts) {
      const id = artifactId(artifact, spec.artifactIdPrefix);
      if (id == null || !canonicalArtifact(artifact)) return consistencyFailure(task, spec);
      ids.push(id);
    }
    if (new Set(ids).size === ids.length) {
      ids.sort((left, right) => left - right);
      const revision = await sha256Json(ids);
      const generated = ids
        .map((id) => recordsById.get(id))
        .filter((record): record is T => record != null);
      const [first, ...rest] = generated;
      if (
        first != null
        && revision != null
        && revision === resultRef.revision
        && generated.length === ids.length
      ) {
        return [first, ...rest];
      }
    }
  }

  return consistencyFailure(task, spec);
}

function consistencyFailure<T extends GeneratedMediaRecord>(
  task: GenerationTaskViewState,
  spec: GeneratedMediaResultSpec<T>,
): never {
  markGenerationConsistencyFailure(
    { taskId: task.taskId, attempt: task.attempt },
    spec.errorMessage,
    task.resultRef,
  );
  throw new Error(spec.errorMessage);
}

function artifactId(artifact: GenerationArtifactRef, prefix: string): number | null {
  if (!artifact.artifact_id.startsWith(prefix)) return null;
  return positiveInteger(artifact.artifact_id.slice(prefix.length));
}

function canonicalArtifact(artifact: GenerationArtifactRef): boolean {
  return artifact.revision === artifact.sha256;
}

function positiveInteger(value: string): number | null {
  if (!/^[1-9]\d*$/.test(value)) return null;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) ? parsed : null;
}
