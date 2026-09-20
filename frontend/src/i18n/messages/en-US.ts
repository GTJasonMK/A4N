import type { Messages } from "./zh-CN";
import { common } from "./en-US-common";
import { nav } from "./en-US-navigation";
import {
  settings,
  settingsAgent,
  settingsForm,
  settingsEmbedding,
} from "./en-US-settings";
import {
  settingsQueue,
  settingsAdmin,
  settingsPrompt,
  settingsTheme,
} from "./en-US-settings-management";
import {
  settingsLogs,
  pageCli,
  pageSession,
  settingsAbout,
  settingsAdvanced,
} from "./en-US-settings-system";
import { auth } from "./en-US-auth";
import { novelProject } from "./en-US-novel-project";
import { novelWorld } from "./en-US-novel-world";
import { novelOverview } from "./en-US-novel-overview";
import { novelAssets } from "./en-US-novel-assets";
import { writing } from "./en-US-writing";
import { visualPlan } from "./en-US-visual-plan";
import { shootingScript } from "./en-US-shooting-script";
import { videoCore } from "./en-US-video-core";
import { videoCanvas } from "./en-US-video-canvas";
import { videoMaterial } from "./en-US-video-material";
import { videoPostprocess } from "./en-US-video-postprocess";
import { videoReference } from "./en-US-video-reference";
import { videoChapter } from "./en-US-video-chapter";
import { comic } from "./en-US-comic";
import { outline } from "./en-US-outline";
import { galgame } from "./en-US-galgame";
import { workflow, errors, agent, a11y, feedback, statusMachine } from "./en-US-runtime";
import { characterDetail } from "./en-US-character";
import { scenes } from "./en-US-scenes";
import { inspiration } from "./en-US-inspiration";
import { units } from "./en-US-units";

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
} satisfies Messages;

export default messages;
