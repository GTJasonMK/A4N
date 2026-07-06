//! 章节生成命令。
//!
//! 保持 `commands::chapter::*` 的 Tauri 命令入口稳定，章节内容、场景、预览和分析实现按职责下沉。

macro_rules! tauri_chapter_progress_command {
    (
        $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) -> $output:ty,
        {$($prepare:stmt);* $(;)?},
        project_id: $project_id:expr,
        total: $total:expr,
        $start_message:expr,
        $error_code:expr,
        |$window:ident, $state:ident, $user_id:ident, $progress:ident| $run:expr,
        $success_summary:expr $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            $window: tauri::Window,
            $state: tauri::State<'_, a4n_runtime::AppState>,
            $user_id: i64,
            $($arg: $arg_ty,)*
            task_id: Option<String>,
        ) -> Result<$output, String> {
            $($prepare;)*
            let progress_task = crate::progress_task::WorkflowProgressTask::new(
                $window.clone(),
                $state.inner(),
                task_id,
                a4n_core::event::channel::TaskType::LlmCall,
                Some($project_id),
                $total,
                $start_message,
            );
            let result = {
                let $progress = &progress_task;
                $run.await
            };
            progress_task.finish_result($state.inner(), &result, $error_code, $success_summary);
            result
        }
    };
}

pub mod analysis;
pub mod content;
pub mod preview;
mod scene;
mod streaming;

pub use self::content::*;
pub use self::scene::*;
pub use self::streaming::*;
