//! The credential prompt of a background job: what is asked for, a masked
//! field and the keys that answer.

use crate::app::state::popup::CredentialPrompt;
use crate::config::localization::t;
use crate::credential_prompt::SecretKind;
use crate::ui::popup::kit::{self, TextBox};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::Frame;

impl DialogView for CredentialPrompt {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let question = match self.kind {
            SecretKind::Password => "credential_prompt_password",
            SecretKind::Passphrase => "credential_prompt_passphrase",
            SecretKind::Certificate => return render_certificate(self, f, cx),
        };
        // The subject goes in first, so the field takes the remaining `{}`.
        let retry = if self.retry {
            format!("{}\n", t("credential_prompt_retry"))
        } else {
            String::new()
        };
        let template = format!(
            "{retry}{}\n{{}}\n\n{}",
            t(question).replacen("{}", &self.subject, 1),
            t("credential_prompt_hint")
        );
        let shown = kit::masked(&self.input);
        TextBox {
            size: (64, 8),
            title: t("credential_prompt_title"),
            border: kit::fg(cx.theme.ui.warning),
            body: kit::prompt_text(&template, &shown, cx.theme),
            body_style: cx.theme.popup_fg(),
        }
        .with_field(kit::field_at(&template))
        .render(f, cx.area, cx.theme);
    }
}

/// The certificate question: server, fingerprint and the keys (no field).
fn render_certificate(prompt: &CredentialPrompt, f: &mut Frame, cx: &ViewCtx<'_>) {
    let (server, fingerprint) = prompt
        .subject
        .split_once('\n')
        .unwrap_or((prompt.subject.as_str(), ""));
    let body = format!(
        "{}\n\nSHA-256:\n{fingerprint}\n\n{}",
        t("credential_prompt_certificate").replacen("{}", server, 1),
        t("credential_prompt_certificate_hint")
    );
    TextBox {
        size: (72, 11),
        title: t("credential_prompt_certificate_title"),
        border: kit::fg(cx.theme.ui.danger),
        body: ratatui::text::Text::from(body),
        body_style: cx.theme.popup_fg(),
    }
    .render(f, cx.area, cx.theme);
}
