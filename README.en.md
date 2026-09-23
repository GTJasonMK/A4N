<p align="center">
  <img src="frontend/src/assets/afn-logo.svg" alt="A4N" width="72" height="72">
</p>

<h1 align="center">A4N</h1>

<p align="center"><strong>Novel writing, comics and video production</strong></p>

<p align="center"><a href="README.md">中文</a> · English</p>

<p align="center">
  <a href="docs/public/getting-started.en.md">Get started</a> ·
  <a href="docs/public/assets/demo.mp4">Watch / download demo</a> ·
  <a href="#core-design">Core design</a> ·
  <a href="https://github.com/GTJasonMK/A4N/issues">Report an issue</a>
</p>

<p align="center"><a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-34699A" alt="Apache License 2.0"></a></p>

A4N helps you write novels, keep track of characters and scenes, and make comics, videos and interactive stories. Start with an idea and outline, or import existing chapters. The app saves chapter analysis and retrieves relevant material before generating the next chapter.

[![The A4N writing desk, with an example chapter and content review side by side](docs/public/assets/writing-desk.webp)](docs/public/assets/demo.mp4)

*The example project Between Mountains and Sea (山海之间). Click the screenshot for a 50-second walkthrough. The demo uses a fictional story, prepared assets and simulated model responses; see the [media notes](docs/public/demo.md).*

> A4N is still in development and currently runs from source. There is no official installer. Setup requires a development environment, PostgreSQL and your own model or API configuration.

## Core design

Open any diagram to view the full-size SVG.

### Chapter analysis and feedback

[![Story feedback: author-triggered analysis creates reusable memory for later writing; revisions require author approval](docs/public/assets/self-feedback.en.svg)](docs/public/assets/self-feedback.en.svg)

After writing a chapter, open the **Analysis** tab and start chapter analysis to extract facts, a summary, character states and foreshadowing. Key results cite the original text so you can check them. The app then updates character, object and scene profiles, adds a retrospective outline and builds retrieval records. Existing formal outlines are kept.

Before generating the next chapter, A4N searches earlier chapters using the outline, writing instructions, relevant characters and unresolved threads. The retrieved context gives priority to character states and events, followed by related summaries and passages.

The writing assistant can also suggest local edits and show the original and proposed text together. It changes the chapter only after you approve. If the original text has changed, the app rejects the outdated suggestion.

*The figure uses a fictional example. This feedback prepares context for writing; it does not retrain the model. Analysis and generated text still need review.*

### Images, comics, video and interactive stories

[![Story sources and profiles connect shared assets to image, comic, video and interactive narrative workflows](docs/public/assets/multimodal.en.svg)](docs/public/assets/multimodal.en.svg)

Images and portraits are saved in the novel project. To make a comic, video or interactive story, select chapters and create a separate derived project. A4N saves the versions of the text, analysis and profiles used for that work. This record is called a **source snapshot**.

After editing the original story, you can compare versions and see which derived content is affected. If you have also edited that content, the app asks you to choose how to handle it.

Video production follows a segment plan. Frames and clips use the same plan, and the app checks that their references match. In first-frame chaining mode, a clip's final frame becomes the next clip's starting image. Supported inputs depend on the workflow and model.

[![The A4N freeform video canvas, connecting prepared footage to trimming, frame extraction and outputs for the example story](docs/public/assets/multimodal-workspace.webp)](docs/public/assets/multimodal-workspace.webp)

*A freeform canvas example: import a prepared clip, trim it and extract a frame. Canvas generation nodes currently require a freeform video project; generate chapter-derived video through the derived project's chapter workflow.*

| What you want to create | What you need |
| --- | --- |
| Novel writing, analysis and optimization | A text model; retrieval feedback also needs Embedding configuration and Qdrant |
| Images, portraits and comic assets | A configured image model/API or a supported local generation service |
| Video generation and assembly | A video model/API; local media processing requires ffmpeg and the corresponding sidecars |
| Interactive narratives | Story scripts and related assets; export requirements depend on the target workspace |

Configure models and services in application settings; see the [configuration guide](docs/public/configuration.en.md). Available formats, resolutions, durations and prices depend on the service.

## Get started

1. Follow the [source setup guide](docs/public/getting-started.en.md) to start the desktop or Web application.
2. Configure a text model and Embedding in settings. Add image and video services as needed.
3. Create a novel project, prepare its blueprint and chapter outline, then write or import the first chapter.
4. Start chapter analysis, inspect the synchronized profiles and threads, then continue writing or review optimization suggestions.

The guide covers installation, the first account, sample chapter text and troubleshooting. Follow it through to writing and analyzing a chapter.

## Backend and task recovery

[![Desktop, browser and CLI entry points converge on shared business logic through Host adapters](docs/public/assets/architecture.en.svg)](docs/public/assets/architecture.en.svg)

The desktop connects to a local Server by default, with Tauri Native mode as another option. The browser and CLI call the same business code through the Server. Separate modules handle API definitions, database operations and model connections.

Resumable tasks save completed results and their inputs. After an interruption, they can continue the remaining steps if those inputs still match. If formal story material, such as the chapter or outline, changes during generation, the app rejects results based on the old version.

| Mechanism | Behavior | Code |
| --- | --- | --- |
| Shared command definitions | Desktop, Web and CLI use the same business command catalog | [Host API contract](crates/host/a4n-api-contract), [parity checks](frontend/package.json) |
| Edit suggestions and conflict checks | Check whether the chapter has changed before writing | [Patch rules and tests](crates/host/a4n-orchestrator/src/optimize/tests/patch_flow.rs) |
| Task checkpoints | Reuse completed steps when the task can resume | [Chapter analysis recovery](crates/host/a4n-orchestrator/src/chapter_analysis/workflow/checkpoint.rs) |
| Source snapshots | Compare story versions and find affected content | [Source differences](crates/platform/a4n-db/src/source_sync/snapshots), [decision guards](crates/platform/a4n-db/src/source_sync/decisions/guards.rs) |

See [Design evidence](docs/public/design-evidence.md) for implementation details and limits, and the [validation notes](docs/public/demo.md) for what the demo tested. A4N uses Rust, Tauri 2, React/TypeScript, PostgreSQL/SeaORM and Qdrant. Development commands are in the [contribution guide](CONTRIBUTING.md) (Chinese).

## Common questions

**Do I pay for models?** A4N source is licensed under Apache 2.0. Third-party APIs, hosting and local inference hardware have their own costs. Check the separate licenses of models and assets.

**Can I use it offline?** External models receive the text, context or media needed for each request. To work offline, run the required models and services locally.

**Where is my work stored?** Structured data lives in the configured PostgreSQL database; media and runtime files live in the configured data directories. Back up both the database and files. The [configuration guide](docs/public/configuration.en.md) explains these boundaries.

**Can scripts or agents use it?** The `a4n` CLI connects to the same Server and offers command discovery, task observation and JSON/JSONL output. See the [CLI guide](docs/public/configuration.en.md#headless-cli).

## Development and feedback

Start with [Contributing](CONTRIBUTING.md); see [release notes](GITHUB_RELEASE.md) for the distinction between source checks, packaging and official distribution. These developer documents are currently in Chinese. For issues, include the version or commit, runtime mode and reproduction steps, with credentials and private content removed. Report security issues privately according to the [security policy](SECURITY.md).

[Apache License 2.0](LICENSE) · [中文版](README.md) · [Start your first story](docs/public/getting-started.en.md)
