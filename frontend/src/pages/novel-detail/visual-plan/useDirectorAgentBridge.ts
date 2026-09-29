import { useEffect, useRef, useState } from "react";
import { controlAgentRun, createAgentThread, getAgentThread, subscribeAgentThreadEvents, type AgentEditorRequest, type AgentEditorSource } from "../../../api/agent";
import { domainAgentBinding, scopeForBinding } from "../../../components/agent/DomainAgentBinding";
import { snapshotCursor } from "../../../utils/agentEventReducer";

/** The originating workspace owns editor requests, even when its Agent panel is closed. */
export function useDirectorAgentBridge(
  projectId: number, chapterId: number | null, ready: boolean, conflicted: boolean,
  source: () => AgentEditorSource | null,
  respond: (request: AgentEditorRequest, runId: string) => Promise<void>,
  report: (failure: unknown) => void,
) {
  const latest = useRef({ source, respond, report });
  const [threadId, setThreadId] = useState<string | null>(null);
  latest.current = { source, respond, report };
  useEffect(() => {
    if (!conflicted || !threadId) return;
    let current = true;
    void (async () => {
      const snapshot = await getAgentThread(threadId);
      if (!current) return;
      for (const run of snapshot.runs) {
        if (["queued", "running", "waiting"].includes(run.status)) {
          await controlAgentRun({ thread_id: threadId, run_id: run.id, action: "pause" });
        }
      }
    })().catch((failure) => { if (current) latest.current.report(failure); });
    return () => { current = false; };
  }, [conflicted, threadId]);
  useEffect(() => {
    const binding = domainAgentBinding("director", projectId, chapterId);
    setThreadId(null);
    if (!binding || !ready) return;
    const controller = new AbortController();
    const viewId = latest.current.source()?.view_instance_id;
    let retry: ReturnType<typeof setTimeout> | undefined;
    let processing: Promise<void> = Promise.resolve();
    const isCurrent = () => !controller.signal.aborted && latest.current.source()?.view_instance_id === viewId;
    const connect = async () => {
      try {
        const thread = await createAgentThread({
          kind: "subagent", capability: "director", scope: scopeForBinding(binding),
          bound_resource: { kind: binding.resourceKind, id: binding.resourceId, label: null, revision: null, hash: null },
        });
        if (!isCurrent()) return;
        setThreadId(thread.id);
        const dispatch = async () => {
          const snapshot = await getAgentThread(thread.id);
          if (!isCurrent()) return;
          for (const event of snapshot.events) {
            const request = event.editor_request;
            if (!request || !event.run_id || request.source.view_instance_id !== viewId
              || request.source.project_id !== projectId || request.source.chapter_id !== chapterId) continue;
            const run = snapshot.runs.find((run) => run.id === event.run_id);
            const execution = snapshot.executions.find((execution) => execution.id === request.execution_id);
            if (run?.status !== "waiting" || run.waiting_reason !== "external"
              || !execution || !["prepared", "awaiting_user", "dispatching"].includes(execution.status)) continue;
            if (!isCurrent()) return;
            await latest.current.respond(request, event.run_id);
          }
        };
        const initial = await getAgentThread(thread.id);
        if (!isCurrent()) return;
        await dispatch();
        await subscribeAgentThreadEvents({ thread_id: thread.id, after_sequence: snapshotCursor(initial), limit: 200 }, (event) => {
          if (!isCurrent() || !event.editor_request) return;
          processing = processing.then(dispatch, dispatch).catch((failure) => {
            if (isCurrent()) latest.current.report(failure);
          });
        }, controller.signal);
        if (isCurrent()) throw new Error("编辑器 Agent 连接已断开，正在重连");
      } catch (failure) {
        if (isCurrent()) {
          latest.current.report(failure);
          retry = setTimeout(() => void connect(), 1500);
        }
      }
    };
    void connect();
    return () => { controller.abort(); clearTimeout(retry); };
  }, [chapterId, projectId, ready]);
}
