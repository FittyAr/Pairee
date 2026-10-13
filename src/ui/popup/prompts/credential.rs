//! The credential prompt of a background job: what is asked for, a masked
//! field and the keys that answer.

use crate::app::state::popup::CredentialPrompt;
use crate::config::localization::t;
use crate::credential_prompt::SecretKind;
use crate::ui::popup::kit::{self, TextBox};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{Frame, style::Color};

impl DialogView for CredentialPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let question = match self.kind {
            SecretKind::Password => "credential_prompt_password",
            SecretKind::Passphrase => "credential_prompt_passphrase",
        };
        // The subject goes in first, so the field takes the remaining `{}`.
        let template = format!(
            "{}\n{{}}\n\n{}",
            t(question).replacen("{}", &self.subject, 1),
            t("credential_prompt_hint")
        );
        let shown = kit::masked(&self.input);
        TextBox {
            size: (64, 8),
            title: t("credential_prompt_title"),
            border: kit::fg(Color::Yellow),
            body: kit::prompt_text(&template, &shown, cx.theme),
            body_style: cx.theme.popup_fg(),
        }
        .with_field(kit::field_at(&template))
        .render(f, cx.area, cx.theme);
    }
}
