//! 视频生成与管理命令

mod config_commands;
mod generation;
mod prompts;
mod records;

pub use self::config_commands::*;
pub use self::generation::*;
pub use self::prompts::*;
pub use self::records::*;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("a4n-tauri crate must live under crates/a4n-tauri")
            .to_path_buf()
    }

    fn read_repo_file(relative: &str) -> String {
        fs::read_to_string(repo_root().join(relative))
            .unwrap_or_else(|err| panic!("failed to read {relative}: {err}"))
    }

    fn collect_frontend_video_invokes() -> BTreeSet<String> {
        let api_dir = repo_root().join("frontend/src/api/video");
        let mut commands = BTreeSet::new();
        for entry in fs::read_dir(&api_dir)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", api_dir.display()))
        {
            let path = entry.expect("video api dir entry must be readable").path();
            if path.extension().and_then(|value| value.to_str()) != Some("ts") {
                continue;
            }
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            let mut remaining = source.as_str();
            while let Some(idx) = remaining.find("apiInvoke") {
                let after_invoke = &remaining[idx + "apiInvoke".len()..];
                let Some(quote_start) = after_invoke.find('"') else {
                    break;
                };
                let after_quote = &after_invoke[quote_start + 1..];
                let quote_end = after_quote.find('"').unwrap_or_else(|| {
                    panic!("unterminated apiInvoke command in {}", path.display())
                });
                let command = &after_quote[..quote_end];
                if is_tauri_command_name(command) {
                    commands.insert(command.to_string());
                }
                remaining = &after_quote[quote_end + 1..];
            }
        }
        commands
    }

    fn collect_registered_video_commands() -> BTreeSet<String> {
        collect_after_prefix(
            &read_repo_file("crates/a4n-tauri/src/command_registry.rs"),
            "commands::video::",
        )
    }

    fn collect_exported_video_commands() -> BTreeSet<String> {
        [
            "crates/a4n-tauri/src/commands/video/config_commands.rs",
            "crates/a4n-tauri/src/commands/video/generation.rs",
            "crates/a4n-tauri/src/commands/video/generation/commands.rs",
            "crates/a4n-tauri/src/commands/video/generation/preview.rs",
            "crates/a4n-tauri/src/commands/video/generation/records.rs",
            "crates/a4n-tauri/src/commands/video/prompts.rs",
            "crates/a4n-tauri/src/commands/video/prompts/records.rs",
            "crates/a4n-tauri/src/commands/video/records.rs",
        ]
        .into_iter()
        .flat_map(|path| collect_exported_command_names(&read_repo_file(path)))
        .collect()
    }

    fn collect_exported_command_names(source: &str) -> BTreeSet<String> {
        let mut names = collect_after_prefix(source, "pub async fn ");
        for prefix in [
            "tauri_state_command!",
            "tauri_input_command!",
            "tauri_id_command!",
            "tauri_id_input_command!",
            "tauri_video_generation_command!",
            "tauri_video_prompt_command!",
        ] {
            names.extend(collect_macro_command_names(source, prefix));
        }
        names
    }

    fn collect_macro_command_names(source: &str, prefix: &str) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        let mut remaining = source;
        while let Some(idx) = remaining.find(prefix) {
            let after_prefix = &remaining[idx + prefix.len()..];
            let after_prefix =
                after_prefix.trim_start_matches(|ch: char| ch.is_whitespace() || ch == '(');
            let end = after_prefix
                .find(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
                .unwrap_or(after_prefix.len());
            let name = &after_prefix[..end];
            if is_tauri_command_name(name) {
                names.insert(name.to_string());
            }
            remaining = &after_prefix[end..];
        }
        names
    }

    fn collect_after_prefix(source: &str, prefix: &str) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        let mut remaining = source;
        while let Some(idx) = remaining.find(prefix) {
            let after_prefix = &remaining[idx + prefix.len()..];
            let end = after_prefix
                .find(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
                .unwrap_or(after_prefix.len());
            let name = &after_prefix[..end];
            if is_tauri_command_name(name) {
                names.insert(name.to_string());
            }
            remaining = &after_prefix[end..];
        }
        names
    }

    fn is_tauri_command_name(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|ch| ch == '_' || ch.is_ascii_lowercase() || ch.is_ascii_digit())
    }

    #[test]
    fn frontend_video_invokes_are_exported_and_registered() {
        let frontend_invokes = collect_frontend_video_invokes();
        let registered = collect_registered_video_commands();
        let exported = collect_exported_video_commands();

        let unregistered: Vec<_> = frontend_invokes.difference(&registered).collect();
        assert!(
            unregistered.is_empty(),
            "frontend video api invokes commands not registered in command_registry.rs: {unregistered:?}"
        );

        let unexported: Vec<_> = frontend_invokes.difference(&exported).collect();
        assert!(
            unexported.is_empty(),
            "frontend video api invokes commands not exported by video/mod.rs: {unexported:?}"
        );

        let registered_without_export: Vec<_> = registered.difference(&exported).collect();
        assert!(
            registered_without_export.is_empty(),
            "command_registry.rs registers video commands not exported by video/mod.rs: {registered_without_export:?}"
        );
    }
}
