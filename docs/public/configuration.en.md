# Configuration and local data

[README](../../README.en.md) · [First run](getting-started.en.md) · [中文](configuration.zh.md)

The full environment template is [`.env.example`](../../.env.example). Keep real credentials in ignored environment files, runtime configuration or the application's settings. Browser and desktop configuration paths can differ; follow the scope shown in the relevant settings page.

## Models and retrieval

| Capability | Configuration and requirements |
| --- | --- |
| Text, analysis and Agents | LLM services in settings; model, URL, key, context capacity and timeout must match the provider |
| Embedding | Local Embedding settings, with separate server defaults available to administrators; select the configuration and set it as default; remote vector dimensions must match |
| Images | Image provider settings; input and output capabilities vary by backend |
| Video | Video provider settings; configure supported resolutions, durations and input modes |
| RAG | Qdrant address and collection, with an Embedding identity matching the project's index |

Desktop builds include on-device fastembed without `--full`. When the desktop connects to a local or remote Server, it manages and runs the device's embedding models; browsers, mobile clients and administrator server settings use server-side capabilities. Desktop source builds and execution require the project's pinned ONNX Runtime; see [First run](getting-started.en.md#2-install-runtime-dependencies-and-start).

`A4N_PUBLIC_LLM_*`, `A4N_PUBLIC_IMAGE_*` and `A4N_PUBLIC_VIDEO_*` provide server-side public model defaults; see the template for fields. Personal keys and local overrides follow the scope shown in settings. Never put server credentials into `VITE_*` variables or public examples.

Text, Embedding, image and video requests are handled by their respective providers. External services receive the required text, retrieval inputs or media. Costs, content handling and model licensing follow the service's terms. Local-first does not promise that every provider runs offline or keeps requests on your machine.

## Runtime addresses and isolation

| Variable | Purpose |
| --- | --- |
| `A4N_DATABASE_URL` | PostgreSQL connection string, required by desktop and Web |
| `A4N_JWT_SECRET` | Runtime JWT secret |
| `A4N_BOOTSTRAP_ADMIN_USERNAME/EMAIL/PASSWORD` | Initial administrator for an empty database; all three are required together |
| `A4N_WEB_ADDR` | Server listener; the desktop's local Server requires loopback |
| `VITE_PORT` | Development frontend port, default `5173` |
| `A4N_POSTGRES_PORT` | Development Compose PostgreSQL port, default `5432` |
| `A4N_FRONTEND_DIST` | Frontend build directory served by the Server |
| `A4N_QDRANT_URL` | Qdrant gRPC address; the template uses `http://127.0.0.1:6334`, not the HTTP management port |
| `A4N_QDRANT_COLLECTION` | Physical collection for this environment |
| `A4N_ONNXRUNTIME_ROOT` | Complete ONNX Runtime installation matching the pinned version |

With no explicit address, the desktop's local-server mode selects an available port. Explicit addresses stay fixed and report conflicts; `127.0.0.1:0` requests an OS-assigned port. Web deployments need a fixed address for their reverse proxy.

Give parallel worktrees separate databases, ports, Qdrant collections and data paths. One-off checks must not connect to an active writing database. Do not update the stable debugging baseline while its watcher is running.

## Files and backup

| Content | Location and handling |
| --- | --- |
| Chapters, profiles, tasks and other structured data | PostgreSQL configured by `A4N_DATABASE_URL` |
| Media and runtime files | `A4N_DATA_DIR`; the local source default is `data/` |
| Cache / temporary files / logs | `A4N_CACHE_DIR` / `A4N_TEMP_DIR` / `A4N_LOG_DIR` |
| ONNX Runtime | Default `~/.local/share/a4n/onnxruntime/<version>`, managed by the pinned installer |
| Qdrant index | The configured Qdrant instance and collection |

Chapters and profiles are not stored in a SQLite database under `data/`. Copying only the media directory does not back up a complete project, and a database dump alone does not preserve file assets. Back up the database, media and relevant configuration, and verify restoration. Project import/export covers only the scope supported by that feature.

Do not commit credentials, OAuth secrets, logs or original user media. Curated README demonstrations are separate public documentation assets, not runtime data.

## Headless CLI

`a4n` connects to the shared Server Host API, with command discovery, JSON/JSONL output, authorization for dangerous operations and task/workflow observation. `--help` and the Host manifest define the supported commands.

Prepare PostgreSQL and the runtime environment first. Export `A4N_DATABASE_URL`, `A4N_JWT_SECRET` and other required settings into the current process environment; direct Cargo invocations do not use the `.env` loading in `scripts/run.sh`. From the source worktree:

```bash
./scripts/cargo.sh run -p a4n-cli -- local start --full
./scripts/cargo.sh run -p a4n-cli -- --output agent-json capabilities check
./scripts/cargo.sh run -p a4n-cli -- local stop
```

`local start --full` is for a source workspace and builds a Server with embedded capabilities. The CLI runs a separate migration before starting the Server; it does not start PostgreSQL or Docker. Ordinary `local start` uses the catalog's managed sidecars.

With a CLI you have already built or packaged:

```bash
a4n health
a4n --output agent-json capabilities export
a4n config runtime set --input @runtime-config.json
```

`config runtime show/set/clear` manages runtime configuration corresponding to GUI local provider settings. Browser-local settings do not automatically sync to the CLI; save a CLI configuration separately and keep the text model, Embedding model and vector dimensions consistent. Generic `call` commands inject configuration according to the manifest's `full`, `without_secrets` or `none` policy. Preview requests omit provider credentials. These commands do not imply that an official installer has been released.

## Linux desktop video

The Linux desktop entry point defaults `WEBKIT_GST_DISABLE_GL_SINK=1` and `WEBKIT_DISABLE_DMABUF_RENDERER=1` to work around seek flicker and paused-resize rendering issues on some NVIDIA/Wayland stacks. Existing environment values take precedence. Memory copies can increase CPU use; after confirming a fix in the target playback stack, set the relevant value to `0` to test that rendering path.

See [Contributing](../../CONTRIBUTING.md) and [release notes](../../GITHUB_RELEASE.md) for development, contract checks and distribution details.
