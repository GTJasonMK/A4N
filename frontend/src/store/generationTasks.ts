import { create } from "zustand";
import type {
  GenerationTaskHandle,
} from "../api/generated/generationContracts";
import {
  createGenerationTaskViewState,
  markGenerationTaskDetached,
  reduceGenerationTask,
  type GenerationTaskReducerAction,
  type GenerationTaskViewState,
} from "../utils/generationEventReducer";

export const generationTaskKey = (taskId: string, attempt: number): string =>
  `${taskId}:${attempt}`;

interface GenerationTaskStore {
  tasks: Record<string, GenerationTaskViewState>;
  references: Record<string, number>;
  retain: (handle: GenerationTaskHandle) => GenerationTaskViewState;
  dispatch: (key: string, action: GenerationTaskReducerAction) => GenerationTaskViewState | null;
  release: (key: string) => number;
  clear: (key: string) => void;
}

const authoritative = new Map<string, GenerationTaskViewState>();
const pendingFlush = new Set<string>();
let flushTimer: ReturnType<typeof setTimeout> | null = null;

export const useGenerationTasksStore = create<GenerationTaskStore>((set) => ({
  tasks: {},
  references: {},

  retain: (handle) => {
    const key = generationTaskKey(handle.task_id, handle.attempt);
    const current = authoritative.get(key) ?? createGenerationTaskViewState(handle);
    const next = reduceGenerationTask(current, { type: "client_state", state: "attached" });
    authoritative.set(key, next);
    set((state) => ({
      tasks: { ...state.tasks, [key]: next },
      references: {
        ...state.references,
        [key]: (state.references[key] ?? 0) + 1,
      },
    }));
    return next;
  },

  dispatch: (key, action) => {
    const current = authoritative.get(key);
    if (!current) return null;
    const next = reduceGenerationTask(current, action);
    authoritative.set(key, next);
    const immediate = shouldFlushImmediately(current, next, action);
    if (immediate) {
      pendingFlush.delete(key);
      set((state) => ({ tasks: { ...state.tasks, [key]: next } }));
    } else {
      scheduleFlush(key, set);
    }
    return next;
  },

  release: (key) => {
    const nextReferences = Math.max(0, (useGenerationTasksStore.getState().references[key] ?? 0) - 1);
    const current = authoritative.get(key);
    const next = current && nextReferences === 0 ? markGenerationTaskDetached(current) : current;
    if (next) authoritative.set(key, next);
    set((state) => ({
      references: { ...state.references, [key]: nextReferences },
      ...(next ? { tasks: { ...state.tasks, [key]: next } } : {}),
    }));
    return nextReferences;
  },

  clear: (key) => {
    authoritative.delete(key);
    pendingFlush.delete(key);
    set((state) => {
      const tasks = { ...state.tasks };
      const references = { ...state.references };
      delete tasks[key];
      delete references[key];
      return { tasks, references };
    });
  },
}));

function shouldFlushImmediately(
  previous: GenerationTaskViewState,
  next: GenerationTaskViewState,
  action: GenerationTaskReducerAction,
): boolean {
  return (action.type === "event"
      && action.event.event.kind === "progress"
      && action.event.event.data.phase_kind === "save"
      && action.event.event.data.item_key != null)
    || action.type === "snapshot"
    || action.type === "summary"
    || action.type === "client_state"
    || action.type === "consistency_error"
    || action.type === "recovery_state"
    || (!previous.terminalReceived && next.terminalReceived)
    || previous.clientState !== next.clientState
    || previous.recoveryState !== next.recoveryState;
}

function scheduleFlush(
  key: string,
  set: (updater: (state: GenerationTaskStore) => Partial<GenerationTaskStore>) => void,
): void {
  pendingFlush.add(key);
  if (flushTimer) return;
  flushTimer = setTimeout(() => {
    flushTimer = null;
    const keys = [...pendingFlush];
    pendingFlush.clear();
    set((state) => {
      const tasks = { ...state.tasks };
      for (const taskKey of keys) {
        const next = authoritative.get(taskKey);
        if (next) tasks[taskKey] = next;
      }
      return { tasks };
    });
  }, 50);
}

export function getGenerationTaskViewState(
  taskId: string,
  attempt: number,
): GenerationTaskViewState | null {
  return authoritative.get(generationTaskKey(taskId, attempt)) ?? null;
}
