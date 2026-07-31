use std::fs::{self, OpenOptions};
use std::path::PathBuf;

use anyhow::{Context, Result};
use reedline::{
    ColumnarMenu, DefaultCompleter, DefaultPrompt, DefaultPromptSegment, EditCommand, Emacs,
    FileBackedHistory, History, HistoryItem, HistoryItemId, HistorySessionId, KeyCode,
    KeyModifiers, Keybindings, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, SearchQuery,
    Signal, default_emacs_keybindings,
};

use crate::client::contains_sensitive_marker;

pub(crate) enum ShellInput {
    Args(Vec<String>),
    Continue,
    Exit,
}

pub(crate) struct InteractiveShell {
    editor: Reedline,
    prompt: DefaultPrompt,
}

impl InteractiveShell {
    pub(crate) fn new(history_path: Option<PathBuf>, completions: Vec<String>) -> Result<Self> {
        let completer = Box::new(DefaultCompleter::new_with_wordlen(completions, 2));
        let menu = ColumnarMenu::default()
            .with_name("completion_menu")
            .with_columns(4)
            .with_column_padding(2);
        let mut keybindings = default_emacs_keybindings();
        add_completion_keybindings(&mut keybindings);
        let mut editor = Reedline::create()
            .with_completer(completer)
            .with_menu(ReedlineMenu::EngineCompleter(Box::new(menu)))
            .with_edit_mode(Box::new(Emacs::new(keybindings)));
        if let Some(path) = history_path {
            prepare_history_file(&path)?;
            let history =
                FileBackedHistory::with_file(2_000, path).context("打开 CLI shell history 失败")?;
            editor = editor.with_history(Box::new(SensitiveHistory { inner: history }));
        }
        Ok(Self {
            editor,
            prompt: DefaultPrompt::new(
                DefaultPromptSegment::Basic("a4n".into()),
                DefaultPromptSegment::Empty,
            ),
        })
    }

    pub(crate) fn read(&mut self) -> Result<ShellInput> {
        match self.editor.read_line(&self.prompt)? {
            Signal::Success(line) => split_line(&line),
            Signal::CtrlC | Signal::ExternalBreak(_) => Ok(ShellInput::Continue),
            Signal::CtrlD => Ok(ShellInput::Exit),
            Signal::HostCommand(line) => split_line(&line),
            _ => Ok(ShellInput::Continue),
        }
    }
}

fn split_line(line: &str) -> Result<ShellInput> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(ShellInput::Continue);
    }
    let args = shlex::split(line).ok_or_else(|| anyhow::anyhow!("shell 命令引号未闭合"))?;
    if args.is_empty() {
        Ok(ShellInput::Continue)
    } else {
        Ok(ShellInput::Args(args))
    }
}

fn add_completion_keybindings(keybindings: &mut Keybindings) {
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
}

fn prepare_history_file(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建 shell history 目录失败: {}", parent.display()))?;
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let _file = options
        .open(path)
        .with_context(|| format!("创建 shell history 失败: {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        _file
            .set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| format!("设置 shell history 权限失败: {}", path.display()))?;
    }
    Ok(())
}

struct SensitiveHistory {
    inner: FileBackedHistory,
}

impl History for SensitiveHistory {
    fn save(&mut self, item: HistoryItem) -> reedline::Result<HistoryItem> {
        if contains_sensitive_marker(&item.command_line) {
            return Ok(item);
        }
        self.inner.save(item)
    }

    fn load(&self, id: HistoryItemId) -> reedline::Result<HistoryItem> {
        self.inner.load(id)
    }

    fn count(&self, query: SearchQuery) -> reedline::Result<i64> {
        self.inner.count(query)
    }

    fn search(&self, query: SearchQuery) -> reedline::Result<Vec<HistoryItem>> {
        self.inner.search(query)
    }

    fn update(
        &mut self,
        id: HistoryItemId,
        updater: &dyn Fn(HistoryItem) -> HistoryItem,
    ) -> reedline::Result<()> {
        self.inner.update(id, updater)
    }

    fn clear(&mut self) -> reedline::Result<()> {
        self.inner.clear()
    }

    fn delete(&mut self, item: HistoryItemId) -> reedline::Result<()> {
        self.inner.delete(item)
    }

    fn sync(&mut self) -> std::io::Result<()> {
        self.inner.sync()
    }

    fn session(&self) -> Option<HistorySessionId> {
        self.inner.session()
    }
}

#[cfg(test)]
mod tests {
    use super::{ShellInput, split_line};
    use crate::client::contains_sensitive_marker;

    #[test]
    fn shell_tokenizes_without_shell_expansion_and_hides_secrets() {
        let ShellInput::Args(args) =
            split_line("call list_projects --args '{\"page\":1}'").unwrap()
        else {
            panic!("expected args");
        };
        assert_eq!(args[0], "call");
        assert_eq!(args[3], r#"{"page":1}"#);
        assert!(split_line("call x --args '$HOME'").is_ok());
        assert!(split_line("call x --args '").is_err());
        assert!(contains_sensitive_marker(
            "call add_llm_config --args '{\"api_key\":\"x\"}'"
        ));
        assert!(contains_sensitive_marker(
            "call inspect --args '{\"media_token\":\"x\"}'"
        ));
        assert!(!contains_sensitive_marker("call list_projects"));
    }
}
