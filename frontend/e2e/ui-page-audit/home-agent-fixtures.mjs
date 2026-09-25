import { readFile } from "node:fs/promises";

// Synthetic read models for UI review; they do not represent executed model/tool work.
const [novel, settings] = await Promise.all(["novel-fixtures.json", "settings-fixtures.json"].map(async (name) =>
  JSON.parse(await readFile(new URL(name, import.meta.url), "utf8")),
));
const chapter = novel.commands.list_chapters[0];
const user = settings.invoke.get_current_user;
const stamp = (minute) => new Date(Date.UTC(2026, 8, 24, 18, minute)).toISOString();
const chapterRef = {
  kind: "chapter", id: String(chapter.id), label: chapter.title,
  revision: chapter.edit_revision, hash: novel.sourceContentHashes[chapter.id],
};
const projectRef = { kind: "project", id: "1", label: novel.commands.get_project.title, revision: null, hash: null };

function thread(id, kind, title, capability, resource) {
  return {
    id, kind, title, capability, bound_resource: resource, status: "active",
    owner_user_id: user.id, protocol_version: 1, revision: 1,
    default_model_config_id: null, context_summary: null,
    next_event_sequence: 1, next_turn_sequence: 1,
    created_at: stamp(0), updated_at: stamp(20),
    scope: {
      resource_kind: resource?.kind ?? null, resource_id: resource?.id ?? null,
      project_id: resource ? 1 : null, chapter_id: resource?.kind === "chapter" ? chapter.id : null,
      chapter_number: resource?.kind === "chapter" ? chapter.chapter_number : null,
      allowed_capabilities: capability ? [capability] : [], allowed_tools: [],
    },
  };
}

function snapshot(thread, messages) {
  return {
    protocol_version: 1, thread,
    messages: messages.map(([role, content], index) => ({
      id: `${thread.id}-message-${index + 1}`, thread_id: thread.id, role, content,
      status: "complete", metadata: null, created_at: stamp(index + 1), updated_at: null,
      run_id: null, turn_id: null, source_id: null, source_kind: null,
    })),
    approvals: [], delegations: [], events: [], executions: [], inbox: [], memories: [], runs: [], turns: [],
  };
}

function run(id, threadId, status, waitingReason, used, budget) {
  return {
    id, thread_id: threadId, turn_id: `${id}-turn`, attempt: 1,
    status, waiting_reason: waitingReason, budget, budget_limiting_run_id: null,
    steps_used: used.steps, model_calls_used: used.models, tool_calls_used: used.tools,
    tokens_used: used.tokens, tokens_reserved: 0, usage_complete: true,
    context_input_tokens: used.tokens, context_capacity_tokens: 16000,
    cost_micros: null, error: null, lease_owner: null, lease_expires_at: null,
    created_at: stamp(1), started_at: stamp(1), updated_at: stamp(20),
  };
}

function turn(run, goal, trigger = "user") {
  return {
    id: run.turn_id, thread_id: run.thread_id, sequence: 1, trigger, status: "active", goal,
    created_at: stamp(1), updated_at: stamp(20), final_message_id: null, result_refs: [],
  };
}

const main = snapshot(thread("thread-0", "main", "主 Agent · 旧书店项目协作", null, null), [
  ["user", "请核对第一章的行动顺序，保留林岚与周衡各自知道的信息。把需要我确认的正文改动单独列出来。"],
  ["assistant", "先按可观察事实梳理：林岚带着旧车票来到门外，周衡把信推到灯下；齿轮暗记与怀表建立联系，墙钟仍停在九点十七分。\n\n这些信息能够支持一次核验行动，但还不能证明父亲的去向。后续建议会区分事实、人物推测和待确认修改。"],
  ["user", "不要把墙钟和怀表合成一个道具。场景切换也要交代她从前门到柜台、再到后门的路径。"],
  ["assistant", "两件道具保留不同作用：墙钟建立房间里的时间矛盾，怀表作为可携带、可交接的证据。\n\n空间顺序沿前门、柜台和后门展开。对话插在林岚观察和移动之间，避免人物凭空改变位置；她尚不能确认的内容会继续留白。"],
  ["user", "先给我两处局部修改建议，写入正文前要由我确认；同时让子线程补一份场景检查清单。"],
    ["assistant", "已有两处局部建议：一处收紧进门前的重复心理描写，另一处把拆信后的追问接到后门敲击声。\n\n当前正文没有写入这些建议。轨迹栏中等待确认的是启动正文优化子任务，写入仍需审阅；既有子线程的场景检查清单受当前预算约束，可以单独查看。"],
]);
const mainRun = run("home-main-run", main.thread.id, "waiting", "approval",
  { steps: 4, models: 1, tools: 1, tokens: 1240 },
  { max_steps: 20, max_model_calls: 10, max_tool_calls: 10, max_tokens: 20000, max_children: 2 },
);
main.runs = [mainRun];
main.turns = [turn(mainRun, "核对第一章并提交两处待审阅修改")];
main.thread.next_turn_sequence = 2;

const executionBase = {
  created_at: stamp(4), updated_at: stamp(12), run_id: mainRun.id,
  effect: null, error: null, replay_policy: null, request_digest: null,
  resource_refs: [chapterRef], result_refs: [], result_summary: null,
};
main.executions = [
  { ...executionBase, id: "home-execution-read", sequence: 1, kind: "tool_call", status: "succeeded", tool_name: "list_chapters", effect: "read" },
  { ...executionBase, id: "home-execution-model", sequence: 2, kind: "model_call", status: "succeeded", tool_name: null, result_summary: "整理两处局部建议，尚未写入正文。" },
  { ...executionBase, id: "home-execution-delegate", sequence: 3, kind: "delegation", status: "running", tool_name: "delegate" },
  { ...executionBase, id: "home-execution-write", sequence: 4, kind: "delegation", status: "awaiting_user", tool_name: "delegate", effect: "create_child_run", replay_policy: "reconcile_required" },
];
main.approvals = [{
  id: "home-approval-chapter", execution_id: "home-execution-write", command: "delegate",
  status: "pending", decision: null, decided_at: null, decided_by_user_id: null,
  created_at: stamp(12), expires_at: null, args_digest: null,
  expected_effect: "将第一章局部修改方案交给正文优化子线程，写入仍需审阅", resource_refs: [chapterRef], resource_revision: chapter.edit_revision,
  reason: "请确认启动正文优化子任务，核对道具、人物知识边界与动作衔接；本次确认不会直接写入正文。",
}];

const budget = snapshot(thread("thread-1", "subagent", "场景检查清单 · 等待追加预算", "general", projectRef), [
  ["user", "为旧书店、阁楼和第三站台整理可核验的场景检查项，保持已有正文不变。"],
  ["assistant", "已整理的检查方向包括入口位置、光线变化、道具交接和人物视线。\n\n当前预算已用完，清单尚未继续细化，也没有应用正文修改。可以查看已用额度，调整本次运行的上限后再继续。"],
]);
const budgetRun = run("home-budget-run", budget.thread.id, "paused", "budget",
  { steps: 12, models: 8, tools: 4, tokens: 6000 },
  { max_steps: 12, max_model_calls: 8, max_tool_calls: 4, max_tokens: 6000 },
);
budget.runs = [budgetRun];
budget.turns = [turn(budgetRun, "补充旧书店项目场景检查清单", "delegation")];
budget.thread.next_turn_sequence = 2;
main.delegations = [{
  id: "home-delegation-budget", capability: "general", status: "running",
  parent_run_id: mainRun.id, parent_execution_id: "home-execution-delegate",
  child_thread_id: budget.thread.id, child_run_id: budgetRun.id, child_turn_id: budgetRun.turn_id,
  task: "根据既有正文补充场景连续性检查清单；不要直接改写正文。",
  success_criteria: ["入口、光线与道具交接有具体检查项", "标明待核验信息"],
  scope: budget.thread.scope, result_refs: [], acceptance_error: null, acceptance_summary: null,
  created_at: stamp(8), updated_at: stamp(20),
}];

main.events = [
  ["message_queued", "收到用户的第一章核对请求。", null],
  ["run_queued", "开始按既有正文检查行动顺序。", null],
  ["execution_prepared", "准备读取第一章的已保存正文。", "home-execution-read"],
  ["execution_succeeded", "第一章正文读取完成。", "home-execution-read"],
  ["execution_succeeded", "局部修改建议已整理，尚未写入。", "home-execution-model"],
  ["execution_prepared", "场景检查清单已交给子线程。", "home-execution-delegate"],
  ["execution_prepared", "准备启动正文优化子任务，等待用户审阅。", "home-execution-write"],
  ["run_waiting", "等待用户确认启动正文优化子任务。", null],
].map(([kind, text, executionId], index) => ({
  protocol_version: 1, thread_id: main.thread.id, run_id: mainRun.id, turn_id: mainRun.turn_id,
  sequence: index + 1, kind, payload: { text }, created_at: stamp(index + 2),
  actor_kind: index === 0 ? "user" : "system", actor_id: index === 0 ? String(user.id) : null,
  message_id: index === 0 ? main.messages[0].id : null,
  execution_id: executionId, delegation_id: executionId === "home-execution-delegate" ? "home-delegation-budget" : null,
  editor_request: null,
}));
main.thread.next_event_sequence = main.events.length + 1;

export const directorSnapshot = snapshot(
  thread("a28524c7-10ea-4cdc-905d-4d2477eb7f13", "subagent", "雨夜来信 · 导演编排讨论", "director", chapterRef),
  [
    ["user", "先讨论第一章旧书店的站位。林岚从前门进入，周衡在柜台后，信件和怀表的出现顺序要清楚。"],
    ["assistant", "开场先建立门外、柜台和后门的关系。林岚停在门槛外观察，周衡把信推到灯下，两人之间保留可见距离。\n\n镜头沿她的注意力转向信封暗记，再回到周衡袖口与柜台下的地图；每次切换都有明确观察动机。"],
    ["user", "保留墙钟和怀表的区别。不要用一个反应镜头暗示林岚已经知道父亲在哪。"],
    ["assistant", "墙钟保留在环境全景中，怀表用单独近景呈现缺齿暗记。林岚的反应落在迟疑、核对和追问上，保持知识边界。\n\n本轮停在编排讨论，未提交镜头或素材修改；下一步可分别核对场景、视线和剪辑接点。"],
  ],
);

export const homeAgentFixtures = {
  list_my_project_invites: [
    { id: 8101, project_title: "旧车站档案补记：关于迟到信件与沿海邮路的协作长篇", inviter_username: "正文协作者", invitee_username: user.username, role: "editor" },
    { id: 8102, project_title: "第三站台 · 场景资料核对", inviter_username: "场景审阅者", invitee_username: user.username, role: "viewer" },
  ],
  agentSnapshots: [main, budget, directorSnapshot],
};
