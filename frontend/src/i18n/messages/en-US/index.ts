import type { Messages } from "../zh-CN";
import { common } from "./common";
import { nav } from "./navigation";
import {
  settings,
  settingsAgent,
  settingsForm,
  settingsEmbedding,
} from "./settings";
import {
  settingsQueue,
  settingsAdmin,
  settingsPrompt,
  settingsTheme,
} from "./settings-management";
import {
  settingsLogs,
  pageCli,
  pageSession,
  settingsAbout,
  settingsAdvanced,
} from "./settings-system";
import { auth } from "./auth";
import { novelProject } from "./novel-project";
import { novelWorld } from "./novel-world";
import { novelOverview } from "./novel-overview";
import { novelAssets } from "./novel-assets";
import { writing } from "./writing";
import { visualPlan } from "./visual-plan";
import { shootingScript } from "./shooting-script";
import { videoCore } from "./video-core";
import { videoCanvas } from "./video-canvas";
import { videoMaterial } from "./video-material";
import { videoPostprocess } from "./video-postprocess";
import { videoReference } from "./video-reference";
import { videoChapter } from "./video-chapter";
import { comic } from "./comic";
import { outline } from "./outline";
import { galgame } from "./galgame";
import { workflow, errors, agent, a11y, feedback, statusMachine } from "./runtime";
import { characterDetail } from "./character";
import { scenes } from "./scenes";
import { inspiration } from "./inspiration";
import { units } from "./units";

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
