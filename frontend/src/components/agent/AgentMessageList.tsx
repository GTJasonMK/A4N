import type { AgentMessageView } from "../../api/agent";
import {
  AgentMessageRow,
  agentToolCallIds,
  agentToolResultCallId,
} from "./AgentMessageRow";
import type { Locale } from "../../i18n";

interface AgentMessageListProps {
  messages: AgentMessageView[];
  locale: Locale;
  agentLabel?: string;
}

interface AgentMessageGroup {
  message: AgentMessageView;
  toolResults: AgentMessageView[];
}

export function AgentMessageList({
  messages,
  locale,
  agentLabel,
}: AgentMessageListProps) {
  const groups = groupAgentMessages(messages);
  return (
    <div className="space-y-2.5">
      {groups.map(({ message, toolResults }) => (
        <AgentMessageRow
          key={message.id}
          message={message}
          locale={locale}
          agentLabel={agentLabel}
          toolResults={toolResults}
        />
      ))}
    </div>
  );
}

export function groupAgentMessages(
  messages: AgentMessageView[],
): AgentMessageGroup[] {
  const groups: AgentMessageGroup[] = [];
  const pendingByCallId = new Map<string, AgentMessageGroup>();

  for (const message of messages) {
    if (message.role === "tool") {
      const callId = agentToolResultCallId(message);
      if (callId) {
        const group = pendingByCallId.get(callId);
        if (group) {
          group.toolResults.push(message);
          pendingByCallId.delete(callId);
          continue;
        }
      }
    }

    const group: AgentMessageGroup = { message, toolResults: [] };
    groups.push(group);
    for (const callId of agentToolCallIds(message)) {
      pendingByCallId.set(callId, group);
    }
  }

  return groups;
}
