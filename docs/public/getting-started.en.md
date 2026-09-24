# Run from source and create your first chapter

[Back to README](../../README.en.md) · [中文](getting-started.zh.md) · [Configuration and local data](configuration.en.md)

The current version runs from source. This guide starts with Web login and chapter analysis, then covers the desktop entry point. Run relative commands from your **development worktree root**. With the repository's multi-worktree layout, use `work/debug`, `work/integration` or your own `work/agent/<topic>`, not the Git coordination root.

## 1. Prepare the environment

- Rust toolchain; CI currently uses Rust 1.95.0.
- Node.js 22, npm and Git.
- PostgreSQL 16+. The commands below use the development Docker Compose database and require Docker with Compose.
- Linux desktop builds also need WebKitGTK, GTK, Ayatana AppIndicator and related development libraries. Follow the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/).

Obtain repository access and check out the source. From your worktree:

```bash
cp .env.example .env
npm --prefix frontend ci
```

Edit `.env`, uncomment the relevant entries and set real values:

```dotenv
A4N_DATABASE_URL=postgres://a4n:a4n@127.0.0.1:5432/a4n
A4N_JWT_SECRET=replace-with-a-long-random-secret
A4N_BOOTSTRAP_ADMIN_USERNAME=writer
A4N_BOOTSTRAP_ADMIN_EMAIL=writer@example.invalid
A4N_BOOTSTRAP_ADMIN_PASSWORD=replace-with-a-strong-initial-password
```

Replace every `replace-with-…` placeholder before starting. `openssl rand -hex 32` can generate a JWT secret. The database URL matches the repository's development Compose configuration; for an existing PostgreSQL instance, supply your own connection string and create the database first. Keep `.env` out of Git.

Parallel worktrees need separate databases, Web/Vite ports, Qdrant collections and data directories. See [Runtime addresses and isolation](configuration.en.md#runtime-addresses-and-isolation).

## 2. Install runtime dependencies and start

```bash
./scripts/dev/dev-db.sh up
./scripts/dev/setup-onnxruntime.sh
./scripts/dev/setup-qdrant.sh
./scripts/run.sh --server
```

- `tools/onnxruntime/VERSION` pins ONNX Runtime. Its default location is `~/.local/share/a4n/onnxruntime/<version>`, separate from Cargo caches. Build scripts stop if the required version is missing.
- Qdrant uses the project's pinned version. The launcher reuses a healthy service at the configured address or starts the installed local binary. Configure a separate address and storage for parallel environments.
- The first build can take time. The entry point checks the database, JWT, frontend and required runtime dependencies. Resolve its explicit errors before retrying.
- Open the Web address printed by the launcher. The default API/static application address is `http://127.0.0.1:8080`; use the actual log output if you configured another address.
- On an empty database, the three `A4N_BOOTSTRAP_ADMIN_*` values create the initial administrator. Log in with that account. Existing users prevent bootstrap from running again; it does not reset passwords.

## 3. Configure creative models

1. Open **Settings → LLM services → Add LLM configuration** (设置 → LLM 服务 → 新增 LLM 配置). Enter a supported model, URL and API key, save the local configuration and set it as default. Choose the Agent protocol supported by your model; the demo uses a compatible service with native tool calling.
2. Open **Settings → Embedding models** (设置 → 嵌入模型). Create a supported Embedding configuration. For remote services, the model and vector dimensions must match the service. Select the configuration and **set it as default**. Administrators can also maintain server defaults under **Administrator → Configuration → Embedding**; local overrides and server defaults have separate scopes.
3. Ensure Qdrant is available before analysis that synchronizes retrieval data. If an existing project's Embedding model or dimensions change, follow the UI's index rebuild instructions.
4. Image and video providers can be configured later; they are not required to enter the first chapter.

Models shown in the demo are deterministic test-provider models, not names to send to an external service. For your own writing, use models actually supported by your provider.

## 4. Complete a chapter and its feedback

From the project list, choose **Create project → Free creation** (创建项目 → 自由创作). Name the project **Between Mountains and Sea** (山海之间), and prepare its blueprint, characters and chapter outline manually or with a model. In **Chapters → Import chapter** (已生成章节 → 导入章节), enter chapter `1`, title **The Unsent Letter**, and this fictional example:

> Rain slanted down the last train's windows as Lin Lan read the unstamped letter for the third time. Her name was on the front; the date on the back was ten years in the future. Enter the old station's clock tower before midnight, or the city will lose seven minutes. The handwriting was identical to hers.
>
> As the train rounded the abandoned platform, the lights went out. A mechanical bell sounded in the darkness. Every phone lit up at once, its clock jumping from 23:42 to 23:35. The passengers repeated their earlier movements without noticing. Only the old watch in Lin Lan's palm moved differently: its second hand turned a full circle backwards.
>
> She got off at the old station. Beyond the rain, the tower's four faces showed different times; the slowest was exactly seven minutes behind. Metal clattered in the hall. Zhou Heng, a watchmaker carrying a toolbox, emerged from a sealed passage. He lost his composure when he saw the gear mark on the envelope.
>
> “That mark should have been burned ten years ago,” he said.
>
> Lin Lan kept the letter and unfolded its last page. A fresh line appeared in bleeding ink: Do not trust whoever sets your time right. A second bell rang from the tower. An advertising screen rewound seven seconds, but the shadow behind Zhou Heng did not move with it. Lin Lan closed her watch and entered the passage leading underground. The letter's first prediction had come true. Eighteen minutes remained until midnight.

After saving, open the writing desk and start analysis from the **Analysis** (分析) tab. Wait for the task to finish, inspect the chapter's material synchronization status, and review extracted facts, characters and threads against the text. Results vary by model.

For the next scene, use an outline or writing instruction that involves the watch and inspect the prepared context and resulting text. From the **Text** (正文) tab, open **Writing assistant → Optimize** (写作助手 → 优化), ask for better expression and pacing while preserving facts, and review the proposed patches before applying them.

Completion checks:

- Reopening the chapter shows the saved text.
- Analysis and material synchronization actually completed. If a step failed, inspect the cause; resume or retry only when the UI permits it.
- Profiles and threads correspond to the source. Extracted model output still needs review.
- Optimization does not rewrite the chapter before approval. Suggestions based on changed source text need a fresh review.

## 5. Desktop, CLI and additional workflows

Once the same environment is ready, start the desktop with:

```bash
./scripts/run.sh
```

For a stable debugging process without file-watcher restarts:

```bash
./scripts/run.sh --no-hot
```

The desktop uses a local Server by default. With `A4N_WEB_ADDR` unset, it prefers `127.0.0.1:8080` and chooses a free port if occupied. An explicitly configured address stays fixed and reports conflicts. See `./scripts/run.sh --help` for Native mode and other options.

Continue into image, comic, video or interactive narrative workspaces from the project or its derivation actions. Configure the required services and preserve source relationships. Generate chapter-derived video through the derived project's chapter workflow. Freeform video canvases arrange assets separately; canvas generation nodes currently require a freeform video project. See [Headless CLI](configuration.en.md#headless-cli) and the [contribution guide](../../CONTRIBUTING.md) for development commands.

## Troubleshooting

| Symptom | Check first |
| --- | --- |
| Startup stops immediately | The first explicit error: database, JWT, ONNX/Qdrant and platform libraries |
| Initial login fails | All three bootstrap values; whether the database already has users |
| A text request fails | The provider's model name, URL, personal key, quota and timeout; never include keys in an issue |
| Analysis saved but synchronization failed | The failing step, default Embedding configuration, dimensions and Qdrant; preserve the error before deciding whether to resume |
| A port is occupied | Stop your conflicting instance or configure an independent port; do not terminate another person's service |
| Linux video rendering problems | [Linux desktop video settings](configuration.en.md#linux-desktop-video), OS/driver details and actual behavior |

Include the commit, reproduction steps and redacted error when reporting a problem. Use the private channel in the [security policy](../../SECURITY.md) for vulnerabilities.
