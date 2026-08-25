import type { ProjectStatus } from "../../api/project";
import type { MessageTranslator } from "../../i18n";

const STATUS_KEYS: Record<ProjectStatus, Parameters<MessageTranslator>[0]> = {
  draft: "statusMachine.statusLabels.draft",
  blueprint_ready: "statusMachine.statusLabels.blueprintReady",
  part_outlines_ready: "statusMachine.statusLabels.partOutlinesReady",
  chapter_outlines_ready: "statusMachine.statusLabels.chapterOutlinesReady",
  writing: "statusMachine.statusLabels.writing",
  completed: "statusMachine.statusLabels.completed",
};

export const projectStatusLabel = (status: ProjectStatus, t: MessageTranslator): string =>
  t(STATUS_KEYS[status]);
