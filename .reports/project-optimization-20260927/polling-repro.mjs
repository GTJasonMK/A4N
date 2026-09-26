// Bundle the unchanged production component; replace only its external dependencies.
import { createRequire } from "node:module";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const dependencies = process.env.A4N_FRONTEND_DEPENDENCIES ?? resolve(repo, "frontend");
const require = createRequire(resolve(dependencies, "package.json"));
const { build } = require("esbuild");
const component = resolve(repo, "frontend/src/pages/novel-detail/FactFeedbackPanel.tsx");
const output = resolve(repo, ".dev-runtime/project-optimization-20260927");
await mkdir(output, { recursive: true });

const fixture = `
import React from "react";
const t = (key, params) => key + (params ? " " + JSON.stringify(params) : "");
const i18n = { t, locale: "zh-CN" };
export const useI18n = () => i18n;
export const useToast = () => ({ addToast() {} });
export const extractErrorMessage = e => String(e);
export const BookButton = ({children,disabled,onClick,type}) => React.createElement("button", {disabled,onClick,type}, children);
export const BookInput = ({value,onChange,disabled}) => React.createElement("input", {value,onChange,disabled});
export const LoadingSpinner = () => React.createElement("span", null, "loading");
export const RequestPreviewModal = () => null;
export const useProgressTask = () => ({runProgressTask: () => {throw Error("Unexpected mutation");}});
export const updateProject = () => {throw Error("Unexpected mutation");};
export const cancelWorkflowTask = () => {throw Error("Unexpected mutation");};
export const previewProjectFactFeedback = () => {throw Error("Unexpected mutation");};
export const runProjectFactFeedback = () => {throw Error("Unexpected mutation");};
window.calls = [];
export async function getProjectFactFeedback() {
  const call = {id: window.calls.length + 1, start: performance.now(), completed: false};
  window.calls.push(call);
  if (call.id > 1) await new Promise(resolve => setTimeout(resolve, Number(new URLSearchParams(location.search).get("delay"))));
  call.completed = true;
  call.end = performance.now();
  return { interval_chapters: 2, pending_chapters: call.id === 1 ? [1] : [],
    consolidated_chapters: call.id === 1 ? [] : [1], legacy_chapters: [], changes: [],
    workflow_status: call.id === 1 ? "running" : "completed", workflow_kind: "project_fact_consolidation",
    workflow_run_id: 1, can_resume: false, error: null };
}
`;

await build({
  stdin: {
    contents: `import React from "react"; import {createRoot} from "react-dom/client"; import {FactFeedbackPanel} from ${JSON.stringify(component)}; createRoot(document.querySelector("#root")).render(<FactFeedbackPanel projectId={1} canEdit />);`,
    loader: "tsx", resolveDir: repo,
  },
  bundle: true,
  outfile: resolve(output, "bundle.js"),
  define: { "process.env.NODE_ENV": '"development"' },
  plugins: [{name: "read-only-fixture", setup(api) {
    api.onResolve({filter: /^react(?:-dom)?(?:\/.*)?$/}, args => ({path: require.resolve(args.path)}));
    api.onResolve({filter: /^\.\.\//}, args => args.importer === component ? {path: args.path, namespace: "fixture"} : undefined);
    api.onLoad({filter: /.*/, namespace: "fixture"}, () => ({contents: fixture, loader: "jsx"}));
  }}],
});
await writeFile(resolve(output, "index.html"), '<!doctype html><meta charset="utf-8"><title>Fact feedback polling reproduction</title><div id="root"></div><script src="bundle.js"></script>');
console.log(output);
