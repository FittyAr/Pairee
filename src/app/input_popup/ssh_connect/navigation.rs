//! Arrow key and Tab navigation for SSH connection dialog.

use crate::config::settings::SshPreset;

pub const MAX_CURSOR_IDX: usize = 10;

pub fn navigate_up(
    idx: &mut usize,
    selected_preset: &mut Option<usize>,
    presets: &[SshPreset],
    name: &mut String,
    host: &mut String,
    port: &mut String,
    user: &mut String,
    pass: &mut String,
    key_path: &mut String,
) {
    if *idx == 0 {
        if !presets.is_empty() {
            let current_sp = selected_preset.unwrap_or(0);
            let next_sp = if current_sp > 0 {
                current_sp - 1
            } else {
                presets.len() - 1
            };
            *selected_preset = Some(next_sp);
            let p = &presets[next_sp];
            *name = p.name.clone();
            *host = p.host.clone();
            *port = p.port.clone();
            *user = p.username.clone();
            *pass = p.password.clone().unwrap_or_default();
            *key_path = p.key_path.clone().unwrap_or_default();
        }
    } else if (7..=10).contains(&*idx) {
        *idx = 6;
    } else {
        *idx = idx.saturating_sub(1);
    }
}

pub fn navigate_down(
    idx: &mut usize,
    selected_preset: &mut Option<usize>,
    presets: &[SshPreset],
    name: &mut String,
    host: &mut String,
    port: &mut String,
    user: &mut String,
    pass: &mut String,
    key_path: &mut String,
) {
    if *idx == 0 {
        if !presets.is_empty() {
            let current_sp = selected_preset.unwrap_or(0);
            let next_sp = if current_sp + 1 < presets.len() {
                current_sp + 1
            } else {
                0
            };
            *selected_preset = Some(next_sp);
            let p = &presets[next_sp];
            *name = p.name.clone();
            *host = p.host.clone();
            *port = p.port.clone();
            *user = p.username.clone();
            *pass = p.password.clone().unwrap_or_default();
            *key_path = p.key_path.clone().unwrap_or_default();
        }
    } else if *idx == 6 {
        *idx = 7;
    } else if (7..=10).contains(&*idx) {
        *idx = 0;
    } else {
        *idx += 1;
    }
}

pub fn navigate_left(idx: &mut usize) {
    if (7..=10).contains(&*idx) {
        *idx = if *idx > 7 { *idx - 1 } else { 10 };
    } else if *idx > 0 && *idx <= 6 {
        *idx = 0;
    }
}

pub fn navigate_right(idx: &mut usize) {
    if (7..=10).contains(&*idx) {
        *idx = if *idx < 10 { *idx + 1 } else { 7 };
    } else if *idx == 0 {
        *idx = 1;
    }
}

pub fn navigate_tab(idx: &mut usize) {
    *idx = if *idx < MAX_CURSOR_IDX { *idx + 1 } else { 0 };
}

pub fn navigate_backtab(idx: &mut usize) {
    *idx = if *idx > 0 { *idx - 1 } else { MAX_CURSOR_IDX };
}
