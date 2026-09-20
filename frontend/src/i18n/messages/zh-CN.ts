import { common } from "./zh-CN-common";
import { nav } from "./zh-CN-navigation";
import {
  settings,
  settingsAgent,
  settingsForm,
  settingsEmbedding,
} from "./zh-CN-settings";
import {
  settingsQueue,
  settingsAdmin,
  settingsPrompt,
  settingsTheme,
} from "./zh-CN-settings-management";
import {
  settingsLogs,
  pageCli,
  pageSession,
  settingsAbout,
  settingsAdvanced,
} from "./zh-CN-settings-system";
import { auth } from "./zh-CN-auth";
import { novelProject } from "./zh-CN-novel-project";
import { novelWorld } from "./zh-CN-novel-world";
import { novelOverview } from "./zh-CN-novel-overview";
import { novelAssets } from "./zh-CN-novel-assets";
import { writing } from "./zh-CN-writing";
import { visualPlan } from "./zh-CN-visual-plan";
import { shootingScript } from "./zh-CN-shooting-script";
import { videoCore } from "./zh-CN-video-core";
import { videoCanvas } from "./zh-CN-video-canvas";
import { videoMaterial } from "./zh-CN-video-material";
import { videoPostprocess } from "./zh-CN-video-postprocess";
import { videoReference } from "./zh-CN-video-reference";
import { videoChapter } from "./zh-CN-video-chapter";
import { comic } from "./zh-CN-comic";
import { outline } from "./zh-CN-outline";
import { galgame } from "./zh-CN-galgame";
import { workflow, errors, agent, a11y, feedback, statusMachine } from "./zh-CN-runtime";
import { characterDetail } from "./zh-CN-character";
import { scenes } from "./zh-CN-scenes";
import { inspiration } from "./zh-CN-inspiration";
import { units } from "./zh-CN-units";

const messages = {
  common,
  nav,
  settings,
  settingsAgent,
  auth,
  novel: {
    ...novelProject,
    ...novelWorld,
    ...novelOverview,
    ...novelAssets,
  },
  writing,
  visualPlan,
  shootingScript,
  video: {
    ...videoCore,
    ...videoCanvas,
    ...videoMaterial,
    ...videoPostprocess,
    ...videoReference,
    ...videoChapter,
  },
  comic,
  outline,
  galgame,
  workflow,
  errors,
  agent,
  a11y,
  feedback,
  statusMachine,
  characterDetail,
  scenes,
  inspiration,
  units,
  settingsForm,
  settingsEmbedding,
  settingsQueue,
  settingsAdmin,
  settingsPrompt,
  settingsTheme,
  settingsLogs,
  pageCli,
  pageSession,
  settingsAbout,
  settingsAdvanced,
} as const;

type MessageShape<T> = {
  readonly [K in keyof T]: T[K] extends string
    ? string
    : T[K] extends Record<string, unknown>
      ? MessageShape<T[K]>
      : never;
};

export type Messages = MessageShape<typeof messages>;
export default messages;
