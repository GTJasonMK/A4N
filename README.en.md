<p align="center">
  <img src="frontend/src/assets/afn-logo.svg" alt="A4N" width="72" height="72">
</p>

<h1 align="center">A4N</h1>

<p align="center"><strong>From novel writing to comics, video and interactive stories</strong></p>

<p align="center"><a href="README.md">中文</a> · English</p>

<p align="center">
  <a href="docs/public/getting-started.en.md">Get started</a> ·
  <a href="docs/public/assets/demo.mp4">Watch / download demo</a> ·
  <a href="#the-workflow-in-screenshots">Screenshots</a> ·
  <a href="#a4n-at-a-glance">Features</a> ·
  <a href="#creative-workflows">Workflows</a> ·
  <a href="#core-design">Core design</a> ·
  <a href="https://github.com/GTJasonMK/A4N/issues">Report an issue</a>
</p>

<p align="center"><a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-34699A" alt="Apache License 2.0"></a></p>

A4N brings novel writing, story knowledge and multimedia adaptation into one creative system. Start from an idea or import an existing work, develop its blueprint, outlines, chapters and profiles, then make comics, videos or interactive stories.

> A4N is still in development and currently runs from source. There is no official installer. Setup requires a development environment, PostgreSQL and your own model or API configuration.

## A4N at a glance

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart TB
  accTitle: A4N feature overview
  accDescr: Novels can become comics, chapter videos and interactive stories through source snapshots. The freeform canvas is an independent entry point. Agents, collaboration, import and export, and recoverable tasks provide support; unconnected blocks do not imply execution order.
  subgraph story[" "]
    direction LR
    writing["Novel writing<br/>Ideas / Blueprint<br/>Outlines / Chapters<br/>Analysis / Profiles<br/>Feedback / Retrieval"]
    writing --> adapted["Adaptations<br/>Source snapshots<br/>Comics /<br/>Chapter video<br/>Interactive stories"]
  end
  subgraph support[" "]
    direction LR
    canvas["Freeform canvas<br/>Text / Images<br/>Audio / Video<br/>Generation /<br/>Media processing"]
    canvas ~~~ tools["Creative tools<br/>Agents / Teams<br/>Import / Export<br/>Task recovery<br/>Multiple clients"]
  end
  story ~~~ support
  style story fill:transparent,stroke:transparent
  style support fill:transparent,stroke:transparent
```

This is a capability map: novels and their adaptations share source references; the canvas also works independently. The sections below explain each workflow.

| What you want to understand | Start here |
| --- | --- |
| Write a novel and keep earlier material organized | [Novel writing](#novel-writing) · [Analysis and fact feedback](#analysis-and-fact-feedback) |
| Prepare images, portraits and production assets | [Images and portraits](#images-and-portraits) |
| Adapt a story into another medium | [Comics](#comics) · [Chapter video](#chapter-video) · [Interactive stories](#interactive-stories) |
| Connect generation and media-processing steps | [Freeform canvas](#freeform-canvas) · [Agents and automation](#agents-and-automation) |
| Manage source changes, collaborators and delivery | [Source synchronization](#source-synchronization) · [Collaboration and import / export](#collaboration-and-import--export) |
| Understand the system and task recovery | [Core design](#core-design) · [Get started](#get-started) |

## The workflow in screenshots

Follow one fictional project, **The Reversing Clock (逆行时钟)**: write a novel, organize its story knowledge, then choose comics, chapter video or interactive storytelling as separate adaptations. The freeform canvas has its own entry point. Key pages appear beside their workflows below; the [complete 14-screenshot tour](docs/public/workflow-tour.md) follows the process in order. Click any screenshot to view its original size. The captured interface is in Chinese, with English explanations in the tour.

[![Project list: The Reversing Clock novel, comic, chapter video, interactive story and independent freeform canvas](docs/public/assets/workflow/00-projects.webp)](docs/public/assets/workflow/00-projects.webp)

*Project management: the novel and its adaptations each open their own workspace; the independent canvas supports custom workflows.*

Screenshots use the real application with isolated fictional data, deterministic local test-provider responses and prepared illustrative media. See the [media and validation notes](docs/public/demo.md#新增页面截图--workflow-screenshots) for the scope.

<details>
<summary>View the earlier 50-second Between Mountains and Sea walkthrough</summary>

[![The A4N writing desk, with an example chapter and content review side by side](docs/public/assets/writing-desk.webp)](docs/public/assets/demo.mp4)

*The example project Between Mountains and Sea (山海之间). Click the screenshot for a 50-second walkthrough. The demo uses a fictional story, prepared assets and simulated model responses; see the [media notes](docs/public/demo.md).*

</details>

## Creative workflows

Follow the arrows, with workflows arranged horizontally where practical. Read steps inside a box from top to bottom; in-box arrows preserve step order. Solid arrows show process or data flow; dashed arrows show optional paths. Edit manually, generate individual steps or inspect saved results without running an entire chain at once.

### Novel writing

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: Novel writing from ideas to chapters
  accDescr: Ideas lead to a blueprint and outlines, then the writing desk. Existing chapters can enter the writing desk directly without blueprint generation.
  idea["Idea /<br/>Story concept"] --> plan["Blueprint →<br/>Part outline<br/>→ Formal<br/>chapter outline<br/>Premise /<br/>World / Cast"]
  plan --> chapter["Writing desk<br/>Write /<br/>Generate"]
  imported["Import /<br/>Enter text"] --> chapter
  chapter --> saved["Save chapters<br/>Analyze /<br/>Revise /<br/>Keep writing"]
```

[![Novel writing desk: story blueprint, character context, chapter list and saved Rainy Night Letter chapter](docs/public/assets/workflow/04-writing.webp)](docs/public/assets/workflow/04-writing.webp)

*The writing desk keeps the blueprint, character context, outline and chapter text together. The [complete tour](docs/public/workflow-tour.md) also shows idea development, the blueprint and formal chapter outlines.*

Blueprints, outlines, chapters and profiles can be inspected and edited. Local revisions have a separate review flow:

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: Local text revisions and author approval
  accDescr: A patch proposal records the selected text and its source version. The author previews the diff. Only an approved patch with a matching source version is applied; rejection or an outdated source keeps the original text.
  proposal["Select text<br/>Record version<br/>Propose patch<br/>Preview diff"]
  proposal --> approval{"Approved and<br/>version matches?"}
  approval -->|Yes| applied["Apply reviewed<br/>patch"]
  approval -->|No| retained["Keep original<br/>text"]
```

### Analysis and fact feedback

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: Chapter analysis and accumulated story knowledge
  accDescr: The author starts analysis of saved text. Extracted summaries, facts, events and threads retain source evidence and update profiles, facts and retrieval indexes. Less frequent knowledge is consolidated periodically or manually and may produce author-reviewed future-plot proposals.
  chapter["Saved chapter<br/>Author starts<br/>analysis"] --> material["Extract / Sync<br/>Summary<br/>Facts / Events<br/>Threads<br/>Cite source<br/>Profiles / Index"]
  material -.-> consolidate["Periodic /<br/>Manual review<br/>World facts<br/>Speech traits<br/>Past growth"]
  consolidate -.-> suggestions["Premise /<br/>Future plot<br/>proposals<br/>Author decides"]
```

Analysis updates characters, objects, scenes and story threads; later writing uses this material for the current task:

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: How earlier story material informs the next chapter
  accDescr: The current outline and writing instructions drive retrieval from chapters one to n−1 with a matching embedding identity. Profiles, facts and retrieved material form the context for chapter n. After saving, the author may start another analysis.
  intent["Chapter n<br/>Outline +<br/>Instructions"] --> retrieval["Retrieve<br/>Chapters<br/>1 to n−1<br/>Embedding<br/>ID matches"]
  retrieval --> context["Profiles / Facts<br/>Continuity →<br/>Summary →<br/>Passages"]
  context --> saved["Generate n<br/>Save text<br/>May analyze"]
```

Less frequent updates consolidate after 5 distinct chapters by default; the interval is configurable, and important world-rule changes can trigger earlier consolidation. Formal outlines are preserved; missing outlines receive retrospective ones. Editing old chapters retracts invalid sources while preserving later manual changes. Saving alone makes no model call. See [Fact feedback](docs/public/fact-feedback.zh.md) (Chinese) and the [feedback example](docs/public/assets/self-feedback.en.svg). This manages writing context; it does not retrain the model.

[![Chapter analysis: completed synchronization for the text index, outlines and threads, objects, characters and scenes](docs/public/assets/workflow/05-analysis.webp)](docs/public/assets/workflow/05-analysis.webp)

*Review analysis and synchronization status beside the chapter, including summaries, events, story threads and character changes.*

### Images and portraits

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: Images, character portraits and asset references
  accDescr: General images use custom prompts; portraits use character profiles and portrait prompts. Each has its own generation entry. Existing images or portraits can be uploaded without model generation, then selected and bound under each workspace's rules.
  generation["Generate<br/>Images: prompt<br/>Portrait: profile<br/>+ prompt<br/>Portrait ages"] --> library["Save records<br/>Image / Portrait<br/>Project scope"]
  uploaded["Upload image<br/>or portrait"] --> library
  library --> reuse["Bind under<br/>workspace rules<br/>Comic /<br/>Director / Story"]
  reuse -.-> dependencies["Inspect<br/>asset links"]
```

General images use custom prompts; portraits combine character profiles with portrait prompts. Images and portraits can also be uploaded, retaining project ownership and references. Character portraits support regeneration and age variants. Each workspace applies its asset-selection rules; saved bindings distinguish system references from user-added images.

### Comics

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: Comic production from story sources to page images
  accDescr: Sources and assets pass through preprocessing and an adopted chapter adaptation plan, then pagination, page content and per-page adaptation, followed by storyboarding, reference discovery, prompts and full-page generation.
  source["1 Prepare<br/>Sources / Assets<br/>Preprocess<br/>Adopt chapter<br/>adaptation plan"]
  source --> pages["2 Pages<br/>Paginate →<br/>Page content<br/>Adapt page<br/>Storyboard"]
  pages --> references["3 References<br/>Find needs →<br/>Bind images<br/>System / User"]
  references --> image["4 Page image<br/>Draw prompt<br/>→ Generate"]
```

Stages retain structured artifacts and their sources; page storyboards cover panels, action, dialogue and narration. System references and user-added images are managed separately; reference discovery only identifies needs and matches existing assets. Delete an existing stage artifact before regenerating, with downstream effects previewed before deletion.

[![Comic storyboard planning: two-page stage progress, a saved page storyboard and reference-image controls](docs/public/assets/workflow/07-manga.webp)](docs/public/assets/workflow/07-manga.webp)

*This shows saved storyboards and the reference-needs check. Final page images remain at 0/2; no finished comic images were generated for this tour.*

### Chapter video

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 112, subGraphTitleMargin: { top: 0, bottom: 12 } }
---
flowchart TB
  accTitle: Chapter video from directing to finished output
  accDescr: The author confirms the director plan and prepares a shooting script. A shared segment plan defines prompts and execution recipes. Image-guided recipes prepare the required frames or references; text-only recipes do not require frames. Both validate inputs before generating clips, editing and export.
  subgraph planning["Planning"]
    direction LR
    plan["1 Direct<br/>Sources / Assets<br/>Director plan<br/>Author confirms<br/>Shooting script"] --> segments["2 Plan clips<br/>Shared plan<br/>Prompts / Recipe<br/>Source ID"]
  end
  subgraph production["Production"]
    direction LR
    inputs{"Image<br/>guided?"} -->|Yes| frames["Prepare<br/>frames /<br/>references"]
    inputs -->|Text only| assembly["3 Produce<br/>Check inputs<br/>→ Make clips<br/>Timeline →<br/>Post-process<br/>Export"]
    frames --> assembly
  end
  planning --> production
  style planning fill:transparent,stroke:#888
  style production fill:transparent,stroke:#888
```

Edit director plans manually or with an Agent; formal confirmation remains an author action. Frames and clips share a segment plan, and first-frame chaining requires the preceding clip's tail frame. Upstream edits trigger dependency checks for affected segments. See [Director editing design](docs/public/director-native-editing.md) (Chinese).

[![Chapter video director workspace: a confirmed six-second shot, three image points, timeline and bound prepared illustration](docs/public/assets/workflow/08-director.webp)](docs/public/assets/workflow/08-director.webp)

*The director timeline combines shots, timing and image points. The bound image is a prepared illustration. The [complete tour](docs/public/workflow-tour.md) continues to the shooting script; this session did not generate a finished video.*

### Interactive stories

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: Interactive stories from branches to playback and export
  accDescr: Adapt a source story, design branches, create dialogue and staging, bind assets, then play the story in the built-in runtime or export a WebGAL project.
  branches["Adapt story<br/>Choices /<br/>Jumps"] --> script["Scenes /<br/>Dialogue<br/>Staging /<br/>Visual plan"]
  script --> assets["Plan / Bind<br/>Backgrounds<br/>Portraits<br/>Images / Audio"]
  assets --> play["Built-in<br/>runtime<br/>Play / Choose"]
  assets --> export["Export<br/>WebGAL"]
```

Edit scenes, lines, order and choices; plan or generate image assets and import audio. Playback and export use the saved story and its asset bindings.

[![Interactive story editor: five nodes, five connections, one choice branch and dialogue editing](docs/public/assets/workflow/10-interactive.webp)](docs/public/assets/workflow/10-interactive.webp)

*The same chapter becomes a story with choices. Inspect its branches on the left and edit node dialogue and options on the right.*

### Freeform canvas

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart LR
  accTitle: Freeform canvas text and image generation example
  accDescr: Text input passes through an LLM to form a prompt, then image generation and a typed image output. This is one valid example. Video generation separately requires a text prompt and any image inputs required by its selected mode.
  text["Text<br/>input"] --> llm["LLM<br/>Prompt"]
  llm --> image["Image<br/>generation"]
  image --> output["Image<br/>output"]
```

Media processing connects matching input types. This example trims video and extracts a frame:

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart LR
  accTitle: Freeform canvas video processing example
  accDescr: An imported video passes through a trim node, then can feed either video output or frame extraction. Extracted frames are images and connect to image output. Images and audio cannot directly enter a video trim node.
  imported["Import<br/>video"] --> trim["Trim<br/>video"]
  trim --> video["Video<br/>output"]
  trim --> frame["Extract<br/>frame"]
  frame --> image["Image<br/>output"]
```

These are two possible workflows. Text, image, audio and video outputs have separate node types. Video generation requires a text prompt; prompt-only, first-frame, first-and-last-frame and reference-image modes have different input requirements. An image alone cannot replace the prompt. The canvas is an independent project. Its generation nodes currently serve freeform video projects; chapter-derived video uses the chapter workflow above.

[![Freeform canvas: prepared video feeds trimming, video output and frame extraction, with all four execution nodes completed](docs/public/assets/workflow/12-canvas.webp)](docs/public/assets/workflow/12-canvas.webp)

*This independent canvas actually trimmed the prepared video, extracted a frame and saved its outputs. The status shows 4/4 execution nodes completed; the input node remains ready. This demonstrates media processing without calling a video-generation model.*

## Agents and automation

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: Agent execution from a creative goal to reviewable results
  accDescr: A domain session binds projects and resources, reads context, and discovers tools and skills. Each call is checked against permissions, execution mode and approval policy. Disallowed or rejected calls do not execute. Authorized calls can edit drafts, start tasks or delegate and review subtasks.
  goal["Goal /<br/>Domain session<br/>Project /<br/>Resources<br/>Revise / Direct<br/>Canvas"]
  goal --> policy["Context /<br/>Tools / Skills<br/>Permissions<br/>Mode / Policy<br/>Author approval<br/>if required"]
  policy --> allowed{"Allow?"}
  allowed -->|No| blocked["Reject call<br/>Record reason"]
  allowed -->|Yes| result["Run tools<br/>Edit / Start<br/>Review results<br/>Delegate /<br/>Review subtasks"]
```

| Execution mode | Constraints on writes |
| --- | --- |
| Advice `advice_only` | Write tools are forbidden; read, analyze and suggest only |
| Review writes `review_writes` | Usually requires approval; `edit_article` only stores a reviewable patch and does not write chapter text |
| Autonomous `autonomous` | Permissions and tool policies still apply; some actions still need author approval |

Agent sessions expose tasks, pending approvals and cancellation of running tasks. The CLI also discovers commands, submits tasks and observes JSON/JSONL events. Adopting chapter patches and formally confirming director plans remain author actions. Available tools and modes depend on the workspace.

## Source synchronization

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: Synchronizing story changes into derived projects
  accDescr: A derived work retains source snapshots. Later changes produce a diff and dependency preview. The author chooses supported strategies per item, such as keeping, merging or replacing. Applying validates targets and updates snapshot links and sync status; unresolved conflicts stay visible and media is not automatically regenerated in bulk.
  snapshot["Source<br/>snapshots<br/>Text / Analysis<br/>Profiles"] --> diff["Source change<br/>Compare<br/>with snapshot<br/>Preview edits /<br/>Asset links"]
  diff --> choose["Author choice<br/>per item<br/>Keep / Merge<br/>Replace, etc."]
  choose --> status["Validate →<br/>Apply plan<br/>Update links<br/>Sync status<br/>Show open<br/>conflicts"]
```

Snapshots identify the story version used for production. The asset dependency graph helps inspect sources and artifacts. Source changes do not silently overwrite local edits. Synchronization does not regenerate all media; supported regeneration strategies require an explicit choice, and detaching from the source has its own strategy. The freeform canvas does not thereby gain novel source snapshots.

[![Source-update preview: a comic detects a changed source and offers Keep current and Apply safe updates](docs/public/assets/workflow/14-source-sync.webp)](docs/public/assets/workflow/14-source-sync.webp)

*Adding a sentence to the novel produces one source change in the derived comic. This screenshot stops at comparison and strategy selection; synchronization has not been applied.*

## Collaboration and import / export

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart LR
  accTitle: Project collaboration and role permissions
  accDescr: Owners and authorized managers invite members as managers, editors or viewers. Accepted members access projects under project and family rules, with permissions checked on each operation. Member invitations do not grant ownership.
  owner["Owner /<br/>Authorized<br/>manager"] --> invite["Invite → Accept<br/>Manager<br/>Editor / Viewer"]
  invite --> access["Project / Family<br/>Check each<br/>operation"]
```

Delivering reading files and moving an editable project serve different purposes:

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 4, wrappingWidth: 120 }
---
flowchart LR
  accTitle: Reading files and editable project transfers
  accDescr: A project can export reading files or structured material, or a supported editable project package. Only the package path means importing elsewhere to continue editing. Reading formats are not complete project backups.
  project["Creative<br/>project"] --> purpose{"Read or<br/>edit?"}
  purpose -->|Read| documents["Reading files<br/>TXT / MD<br/>EPUB / PDF<br/>Outlines /<br/>Profiles JSON"]
  purpose -->|Edit| transfer["Supported<br/>editable pack<br/>Project / Family<br/>Import → Edit"]
```

Member permissions apply to projects and related works. Reading files and editable project packages serve different purposes. Package exports include only the supported contents; complete backups still require both PostgreSQL data and media files.

[![Novel export: TXT selected, with a completed download and its file name](docs/public/assets/workflow/13-export.webp)](docs/public/assets/workflow/13-export.webp)

*Delivery produces a local file: the TXT was actually downloaded and checked against the latest saved chapter in full.*

## Core design

### Shared business logic across entry points

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: How entry points reach the shared business core
  accDescr: Desktop default mode, Web and CLI enter the business core through the Server service group. Optional desktop Native mode uses Tauri command adapters. Shared Host API contracts constrain both entries; the contracts are not an independent running service.
  clients["Desktop<br/>default<br/>Web / CLI"] --> server["Server<br/>HTTP / Events"]
  server --> business["Shared core<br/>Writing /<br/>Adaptation<br/>Agents /<br/>Workflows"]
  native["Desktop<br/>Native<br/>Optional"] -.->|Tauri| business
```

Shared Host API contracts constrain both entries. They define interfaces, rather than a separate running service. The Server uses Edge, Control and Runtime / Worker components. The next diagram shows only responsibilities and resources:

```mermaid
---
config:
  flowchart: { nodeSpacing: 20, rankSpacing: 24, padding: 8 }
---
flowchart TB
  accTitle: Data and capability boundaries of the shared business core
  accDescr: Business orchestration uses data access for PostgreSQL, Qdrant retrieval indexes and media files, and runtime adapters for model Providers and media processing. This shows responsibilities and resources, not process deployment or Rust dependencies.
  business["Business orchestration"] --> data["Data / Indexes<br/>PostgreSQL<br/>Qdrant<br/>Media files"]
  business --> runtime["Runtime adapters<br/>Text / Image / Video<br/>ffmpeg · Sidecars"]
```

Arrows show calls or data flow, not direct Rust dependencies. API contracts, business rules, persistence and runtime adapters have separate owners. Enable optional capability packages as needed, and configure text, Embedding, image and video services separately. Models may use local services or external APIs.

### Recoverable tasks and version protection

```mermaid
---
config:
  flowchart: { nodeSpacing: 16, rankSpacing: 12, padding: 2, wrappingWidth: 96 }
---
flowchart LR
  accTitle: Checkpoint recovery and source-version protection
  accDescr: Recovery-capable workflows check input identities per stage. Matching completed results may be reused; otherwise the current stage executes. Successful work passes that stage's write checks, persists results and moves on or finishes. Failures and cancellations retain explicit status; results are not all deferred until the end.
  request["Submit /<br/>Resume task<br/>Stage inputs<br/>Source version<br/>and config"] --> checkpoint{"Reuse?"}
  checkpoint -->|Yes| reuse["Read saved<br/>results"]
  checkpoint -->|No| run["Run stage<br/>Validate<br/>sources<br/>Save artifacts<br/>Checkpoint"]
  reuse --> next["Next stage<br/>or finish"]
  run --> next
```

Recovery depends on the workflow and its state. If all stages already have valid results, the task can finish without executing more stages. Artifacts are saved under each stage's write rules; a checkpoint is not a formal artifact and cannot bypass version protection after chapters, outlines or sources change.

| Situation | Result |
| --- | --- |
| Execution failure or task cancellation | Keep explicit status and errors or cancellation reasons; retain completed stages as the workflow permits |
| Formal source version changes | Reject outdated writes and request a refresh or regeneration |
| Task recovery requested | Recheck checkpoints against current sources and inputs; not every state supports recovery |

See [Design evidence](docs/public/design-evidence.md) for code entry points and limits, and the [validation notes](docs/public/demo.md) for the public demo's narrower scope. The stack is Rust, Tauri 2, React/TypeScript, PostgreSQL/SeaORM and Qdrant.

## Get started

1. Follow the [source setup guide](docs/public/getting-started.en.md) to start the desktop or Web application.
2. Configure the models and services you need in settings.
3. Create a novel project, start with ideas and a blueprint, or enter / import existing chapters.
4. Save and analyze a chapter, check its profiles and feedback, then continue writing or create a multimedia work.

| Capability | What you need |
| --- | --- |
| Writing, analysis, optimization and Agents | A text model / API |
| Earlier-chapter retrieval | Embedding configuration and Qdrant |
| Images, portraits and comic pages | An image model / API or supported local service |
| Video generation and media processing | A video model / API; processing and assembly require ffmpeg and corresponding sidecars |
| Interactive stories | Scripts and related assets; playback and export conditions depend on the workspace |

Configure models and services in application settings; see the [configuration guide](docs/public/configuration.en.md). Formats, resolutions, durations and costs depend on the Provider.

## Common questions

**Do I pay for models?** A4N source is licensed under Apache 2.0. Third-party APIs, hosting and local inference hardware have their own costs. Check the separate licenses of models and assets.

**Can I use it offline?** External models receive the text, context or media needed for each request. To work offline, run the required models and services locally.

**Where is my work stored?** Structured data lives in the configured PostgreSQL database; media and runtime files live in the configured data directories. Back up both the database and files. The [configuration guide](docs/public/configuration.en.md) explains these boundaries.

**Can scripts or agents use it?** The `a4n` CLI connects to the same Server and offers command discovery, task observation and JSON/JSONL output. See the [CLI guide](docs/public/configuration.en.md#headless-cli).

## Development and feedback

Start with [Contributing](CONTRIBUTING.md); see [release notes](GITHUB_RELEASE.md) for the distinction between source checks, packaging and official distribution. These developer documents are currently in Chinese. For issues, include the version or commit, runtime mode and reproduction steps, with credentials and private content removed. Report security issues privately according to the [security policy](SECURITY.md).

[Apache License 2.0](LICENSE) · [中文版](README.md) · [Start your first story](docs/public/getting-started.en.md)
