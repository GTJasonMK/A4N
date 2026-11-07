<p align="center">
  <img src="frontend/src/assets/afn-logo.svg" alt="A4N" width="72" height="72">
</p>

<h1 align="center">A4N</h1>

<p align="center"><strong>From novel writing to comics, video and interactive stories</strong></p>

<p align="center"><a href="README.md">中文</a> · English</p>

<p align="center">
  <a href="docs/public/getting-started.en.md">Get started</a> ·
  <a href="docs/public/assets/demo.mp4">Watch / download demo</a> ·
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
flowchart LR
  accTitle: A4N feature overview
  accDescr: Four capability groups cover novel writing, multimedia adaptation, a freeform canvas and shared creative tools.
  A4N["A4N"] --> writing["Novel writing"]
  writing --> knowledge["Ideas · Blueprint · Outlines<br/>Chapters · Analysis · Profiles<br/>Fact feedback · Retrieval"]
  A4N --> adapted["Multimedia adaptation"]
  adapted --> works["Comics · Video · Interactive stories<br/>Snapshots · Asset references · Sync"]
  A4N --> canvas["Freeform canvas"]
  canvas --> nodes["Text · Images · Audio / Video<br/>Nodes · Generation · Processing"]
  A4N --> platform["Creative tools"]
  platform --> tools["Agents · Collaboration · Import / Export<br/>Recoverable tasks · Multiple entry points"]
```

| What you want to understand | Start here |
| --- | --- |
| Write a novel and keep earlier material organized | [Novel writing](#novel-writing) · [Analysis and fact feedback](#analysis-and-fact-feedback) |
| Prepare images, portraits and production assets | [Images and portraits](#images-and-portraits) |
| Adapt a story into another medium | [Comics](#comics) · [Chapter video](#chapter-video) · [Interactive stories](#interactive-stories) |
| Connect generation and media-processing steps | [Freeform canvas](#freeform-canvas) · [Agents and automation](#agents-and-automation) |
| Manage source changes, collaborators and delivery | [Source synchronization](#source-synchronization) · [Collaboration and import / export](#collaboration-and-import--export) |
| Understand the system and task recovery | [Core design](#core-design) · [Get started](#get-started) |

<details>
<summary>View the writing desk and a 50-second walkthrough</summary>

[![The A4N writing desk, with an example chapter and content review side by side](docs/public/assets/writing-desk.webp)](docs/public/assets/demo.mp4)

*The example project Between Mountains and Sea (山海之间). Click the screenshot for a 50-second walkthrough. The demo uses a fictional story, prepared assets and simulated model responses; see the [media notes](docs/public/demo.md).*

</details>

## Creative workflows

The diagrams follow the creative stages. Edit manually, generate individual steps or inspect saved results without running an entire chain at once.

### Novel writing

```mermaid
flowchart TB
  accTitle: Novel writing from ideas to chapters
  accDescr: Ideas lead to a blueprint and outlines, or existing chapters can be imported. Chapters feed analysis and further writing; local patches require author approval and matching source versions.
  idea["Idea conversation / Story concept"] --> blueprint["Story blueprint<br/>Premise · World · Characters · Relationships"]
  blueprint --> outline["Part outlines → Formal chapter outlines"]
  outline --> chapter["Writing desk<br/>Write manually / Generate a chapter"]
  imported["Enter / Import existing chapters"] --> chapter
  chapter --> analysis["Chapter analysis and fact feedback"]
  analysis --> next["Continue with earlier story knowledge"]
  next --> chapter
  chapter --> proposal["Local revision proposal → Diff preview"]
  proposal --> approval{"Author approves and source version matches?"}
  approval -->|Yes| chapter
  approval -->|No| retained["Keep the original / Request a fresh proposal"]
```

Blueprints, outlines, chapters and profiles can be inspected and edited. Local revisions show the original and proposed text before approval; outdated proposals cannot overwrite changed source text.

### Analysis and fact feedback

```mermaid
flowchart TB
  accTitle: Chapter analysis, accumulated knowledge and writing feedback
  accDescr: Analysis cites source passages, synchronizes current facts each chapter and periodically consolidates knowledge. Retrieval prepares context from earlier chapters; changes to future plotting remain proposals for the author.
  chapter["Saved chapter + Author starts analysis"] --> extract["Extract summaries, facts, events and threads<br/>Key results cite source evidence"]
  extract --> profiles["Per-chapter updates<br/>Characters · Objects · Scenes · Thread progress"]
  extract --> index["Analysis records and retrieval index"]
  profiles --> consolidate["Periodic / Manually triggered consolidation<br/>World facts · Speech traits · Observed growth"]
  consolidate --> suggestions["Premise and future-plot proposals<br/>Author decides whether to adopt"]
  profiles --> context["Current outline + Writing instructions + Prior knowledge"]
  index --> retrieval["Task-specific retrieval<br/>Chapters 1 to n−1 + Matching embedding identity"]
  retrieval --> context
  consolidate --> context
  context --> generation["Generate chapter n<br/>Continuity first, then summaries and passages"]
  generation -.->|Analyze again after saving| chapter
```

Less frequent updates consolidate after 5 distinct chapters by default; the interval is configurable, and important world-rule changes can trigger earlier consolidation. Formal outlines are preserved; missing outlines receive retrospective ones. Editing old chapters retracts invalid sources while preserving later manual changes. Saving alone makes no model call. See [Fact feedback](docs/public/fact-feedback.zh.md) (Chinese) and the [feedback example](docs/public/assets/self-feedback.en.svg). This manages writing context; it does not retrain the model.

### Images and portraits

```mermaid
flowchart TB
  accTitle: Images, character portraits and asset references
  accDescr: Character profiles and creative instructions feed editable prompts. Generated and uploaded images enter project libraries, then workspaces select and bind them with inspectable dependencies.
  profiles["Character profiles + Creative instructions"] --> prompt["Generate / Edit a portrait prompt"]
  prompt --> generation["Image model generation<br/>Character portraits / Creative images"]
  generation --> library["Project image and portrait libraries"]
  uploaded["Upload existing images"] --> library
  library --> reuse["Select or bind assets in each workspace<br/>Comic references · Director image points · Story assets"]
  reuse --> dependencies["Inspect asset dependencies<br/>Trace references and affected artifacts"]
```

Generate or upload images and portraits with project ownership and references retained. Character portraits support regeneration and age variants. Each workspace applies its asset-selection rules; saved bindings distinguish system references from user-added images.

### Comics

```mermaid
flowchart TB
  accTitle: Comic production from story sources to page images
  accDescr: Sources and assets pass through preprocessing, adaptation, page planning, storyboarding, reference binding, drawing prompts and page generation. Intermediate artifacts remain inspectable and editable.
  source["Story sources + Character / Scene / Image assets"] --> prepare["Comic preprocessing and chapter adaptation"]
  prepare --> pages["Pagination and page content → Per-page adaptation"]
  pages --> storyboard["Page storyboard<br/>Panels · Action · Dialogue · Narration"]
  storyboard --> references["Discover and bind references<br/>System portraits + User-added images"]
  references --> prompt["Full-page drawing prompt"]
  prompt --> image["Page image → Inspect / Regenerate"]
```

Stages retain structured artifacts and their sources. Inspect, edit or delete a selected stage, with downstream effects previewed before deletion. System references and user-added images are managed separately.

### Chapter video

```mermaid
flowchart TB
  accTitle: Chapter video from directing to finished output
  accDescr: The author confirms the director plan. A shooting script feeds a shared segment plan used by both frame images and video clips, followed by timeline assembly and post-processing.
  source["Chapter sources + Base assets"] --> director["Director plan<br/>Shots · Timing · Image points · Staging"]
  director --> confirm["Author confirms plan → Shooting script"]
  confirm --> segments["Shared segment plan<br/>Source identity · Execution recipe · References"]
  segments --> frames["Generate / Bind frame images"]
  segments --> prompts["Video prompts and segment settings"]
  frames --> clips["Generate video clips by segment"]
  prompts --> clips
  clips --> assembly["Timeline editing and assembly"]
  assembly --> post["Post-processing / Finished output"]
```

Edit director plans manually or with an Agent; formal confirmation remains an author action. Frames and clips share a segment plan, and first-frame chaining requires the preceding clip's tail frame. Upstream edits trigger dependency checks for affected segments. See [Director editing design](docs/public/director-native-editing.md) (Chinese).

### Interactive stories

```mermaid
flowchart TB
  accTitle: Interactive stories from branches to playback and export
  accDescr: Adapt a source story, design branches, create dialogue and staging, bind assets, then play the story in the built-in runtime or export a WebGAL project.
  source["Story sources and adaptation plan"] --> branches["Branch design<br/>Choices · Conditions · Jumps"]
  branches --> script["Scene scripts and dialogue"]
  script --> staging["Character staging and visual design"]
  staging --> assets["Asset planning and binding<br/>Backgrounds · Portraits · Images · Audio"]
  assets --> play["Built-in story runtime<br/>Advance dialogue / Choose a branch"]
  assets --> export["Export a WebGAL project"]
```

Edit scenes, lines, order and choices; plan or generate image assets and import audio. Playback and export use the saved story and its asset bindings.

### Freeform canvas

```mermaid
flowchart LR
  accTitle: Freeform canvas node workflow
  accDescr: Text and media enter connected generation or processing nodes and end in previews or outputs. The actual workflow follows the nodes connected by the author.
  text["Text input"] --> llm["LLM text node"]
  llm --> image["Image generation"]
  image --> video["Video generation"]
  imported["Import images / Audio / Video"] --> process["Trim · Extract frames · Process media"]
  video --> process
  process --> output["Preview / Output nodes"]
  llm --> output
  image --> output
```

This is an example of connected steps, not a requirement to run every node. The canvas is an independent project. Canvas generation nodes currently serve freeform video projects; chapter-derived video uses the chapter workflow above.

<details>
<summary>View a freeform canvas example</summary>

[![Freeform canvas: imported video, trimming, frame extraction and outputs](docs/public/assets/multimodal-workspace.webp)](docs/public/assets/multimodal-workspace.webp)

The example uses prepared video to show media processing; it does not demonstrate a call to a video-generation model.

</details>

## Agents and automation

```mermaid
flowchart TB
  accTitle: Agent execution from a creative goal to reviewable results
  accDescr: Domain sessions for novel optimization, directing and canvas work read bound context, discover tools and skills, then act within permissions and execution modes. Tasks and approvals remain observable; formal text patches and director confirmation remain author actions.
  goal["Author goal<br/>Novel optimization / Directing / Canvas"] --> session["Domain session bound to project and resources"]
  session --> tools["Read context · Discover tools · Load Skills"]
  tools --> mode{"Permissions and execution mode"}
  mode -->|Suggest| proposal["Read-only analysis / Revision proposals"]
  mode -->|Approval required| review["Author approves a tool operation"]
  mode -->|Autonomous| execute["Execute authorized valid operations"]
  review --> execute
  execute --> result["Draft edits / Generation tasks / Receipts"]
  execute -.-> delegation["Delegate subtasks when needed"]
  delegation -.-> result
  proposal --> monitor["Inspect sessions, tasks and results<br/>Resolve approvals or cancel running tasks"]
  result --> monitor
```

Agents can use project tools and handle long-running tasks. The CLI also discovers commands, submits tasks and observes JSON/JSONL events. Adopting chapter patches and formally confirming director plans remain author actions. Available tools and modes depend on the workspace.

## Source synchronization

```mermaid
flowchart TB
  accTitle: Synchronizing story changes into derived projects
  accDescr: Derived projects retain text, analysis and profile snapshots. Later source changes are compared first; local edits require an author decision, followed by dependency validation and updates to affected content.
  novel["Novel text · Analysis · Profiles"] --> snapshot["Save a source snapshot when deriving a project"]
  snapshot --> derived["Comics / Chapter video / Interactive stories"]
  novel --> changes["Later changes to the original"]
  changes --> diff["Compare source versions<br/>Chapter order · Text · Analysis · Profiles"]
  snapshot --> diff
  diff --> decision{"Local edits in the derived work?"}
  decision -->|Yes| choose["Author chooses how to handle them"]
  decision -->|No| apply["Confirm synchronization"]
  choose --> apply
  apply --> dependencies["Validate asset and downstream dependencies<br/>Update sources / Mark affected artifacts"]
  dependencies --> derived
```

Snapshots identify the story version used for production. The asset dependency graph helps inspect relationships between sources and artifacts. Source changes do not silently overwrite local edits. The freeform canvas does not thereby gain novel source snapshots. See the [multimedia source example](docs/public/assets/multimodal.en.svg).

## Collaboration and import / export

```mermaid
flowchart TB
  accTitle: Project collaboration and delivery
  accDescr: Existing text and project packages enter creative projects. Invited collaborators access them according to roles. Exports provide reading formats or supported editable project and media packages.
  imported["Enter / Import text · Import project packages"] --> project["Creative projects and project families"]
  project --> invite["Invite collaborators → Accept invitation"]
  invite --> roles["Manager / Editor / Viewer<br/>Role-based project access"]
  project --> text["Novel delivery<br/>TXT · Markdown · EPUB · PDF"]
  project --> records["Structured material<br/>Outline / Character JSON"]
  project --> package["Supported project / Family packages<br/>Included material, sources and media"]
  package --> transfer["Import elsewhere / Continue creating"]
```

Member permissions apply to projects and related works. Reading files and editable project packages serve different purposes. Package exports include only the supported contents; complete backups still require both PostgreSQL data and media files.

## Core design

### Shared business logic across entry points

```mermaid
flowchart TB
  accTitle: Entry points, shared business logic, storage and model capabilities
  accDescr: The desktop defaults to a local Server; Web and CLI also enter the shared core through Server adapters. Tauri Native is another entry adapter. Separate boundaries own data, retrieval, files and model services; this is not a Rust dependency graph.
  desktop["Desktop · Tauri 2"] -->|Default| server["Local / Deployed Server"]
  web["Web browser"] --> server
  cli["a4n CLI / Automation"] --> server
  desktop -.->|Optional| native["Tauri Native adapter"]
  server --> host["Shared Host API contracts and entry adapters"]
  native --> host
  host --> business["Business rules and orchestration<br/>Writing · Adaptation · Agents · Workflows"]
  business --> db["PostgreSQL<br/>Chapters · Profiles · Versions · Tasks"]
  business --> rag["Embedding + Qdrant<br/>Earlier-chapter retrieval index"]
  business --> files["Data directory<br/>Images · Audio / Video · Exported files"]
  business --> runtime["Runtime adapters and capability services"]
  runtime --> models["Text / Image / Video Providers<br/>Local services or external APIs"]
  runtime --> media["Media processing<br/>ffmpeg and corresponding sidecars"]
```

Arrows show calls or data flow, not direct Rust dependencies. API contracts, business rules, persistence and runtime adapters have separate owners. Enable optional capability packages as needed, and configure text, Embedding, image and video services separately. See the more detailed [architecture example](docs/public/assets/architecture.en.svg).

### Recoverable tasks and version protection

```mermaid
flowchart TB
  accTitle: Checkpoint recovery and source-version protection
  accDescr: Workflows that support recovery reuse completed steps only with matching input identities. Failures remain visible. A final source-version check rejects results produced from outdated formal material.
  request["Submit task<br/>Record inputs, source versions and runtime config"] --> task["Task ID + Progress / Events"]
  task --> checkpoint{"Reusable completed steps<br/>with matching input identity?"}
  checkpoint -->|Yes| reuse["Reuse completed results"]
  checkpoint -->|No| run["Execute remaining steps"]
  reuse --> run
  run --> saved["Persist stage results and checkpoints"]
  saved --> version{"Formal source version still matches?"}
  version -->|Yes| commit["Commit formal results"]
  version -->|No| conflict["Reject outdated results<br/>Refresh / Regenerate"]
  run -->|Failure or interruption| stopped["Record explicit status and error"]
  stopped -.->|When workflow supports recovery| checkpoint
```

Recovery depends on the workflow and its current state. Checkpoints cannot bypass version checks after chapters, outlines or sources change. Failures, cancellations and conflicts have explicit outcomes.

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
