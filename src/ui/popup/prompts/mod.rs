pub mod confirm;
pub mod file_ops;
pub mod filter;
pub mod help;
pub mod plugin_dialogs;
pub mod ssh;
pub mod system;

use crate::app::context::AppContext;
use crate::app::state::PopupType;
use ratatui::{Frame, layout::Rect};

pub fn render_prompt_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    context: &AppContext,
) -> bool {
    match popup {
        PopupType::Help { .. } => help::render(f, popup, theme, size),

        PopupType::MkDirPrompt { .. }
        | PopupType::CopyPrompt { .. }
        | PopupType::RenMovPrompt { .. }
        | PopupType::ConfirmOverwrite { .. }
        | PopupType::ConfirmDelete { .. }
        | PopupType::WipeConfirm { .. }
        | PopupType::CreateLinkPrompt { .. }
        | PopupType::DescribeFilePrompt { .. }
        | PopupType::CompressPrompt { .. } => file_ops::render(f, popup, theme, size),

        PopupType::ConfirmQuit
        | PopupType::ConfirmInterrupt
        | PopupType::ConfirmReload { .. }
        | PopupType::ConfirmClearHistory { .. }
        | PopupType::SaveSetupConfirm => confirm::render(f, popup, theme, size),

        PopupType::FilePanelFilterPrompt { .. }
        | PopupType::QuickFilterPrompt { .. }
        | PopupType::CopyMoveFilterPrompt { .. } => filter::render(f, popup, theme, size),

        PopupType::SshConnectPrompt { .. } => ssh::render(f, popup, theme, size, context),

        PopupType::CopyProgress { .. }
        | PopupType::ConfirmRetryAsAdmin { .. }
        | PopupType::Error(_)
        | PopupType::Info(_)
        | PopupType::ApplyCommandPrompt { .. }
        | PopupType::SelectGroupPrompt { .. }
        | PopupType::PluginNotify { .. } => system::render(f, popup, theme, size),

        PopupType::PluginInputDialog { .. }
        | PopupType::PluginConfirmDialog { .. }
        | PopupType::PluginWhichPrompt { .. } => plugin_dialogs::render(f, popup, theme, size),

        _ => false,
    }
}
