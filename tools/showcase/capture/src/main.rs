//! Runs the real `pairee` binary in a pseudo-terminal, replays a scenario of
//! key presses and saves the terminal screen (characters and colors) as JSON
//! for `render.py`.
//!
//! Usage: capture <pairee> <cols> <rows> <scenario> <out-dir> [-- <pairee args>...]
//!
//! Scenario lines (blank lines and `#` comments are ignored):
//!   start [args...]   (re)start pairee in the scenario folder with these arguments
//!   keys <key> ...    send keys: F1..F12, Enter, Esc, Tab, Up, Down, Left, Right,
//!                     Home, End, PgUp, PgDn, Ins, Del, Bksp, Space, C-x (Ctrl), CA-x (Ctrl+Alt),
//!                     A-x / A-F7 (Alt), S-F4 (Shift), "text" (typed as is)
//!   wait <ms>         sleep
//!   shot <name>       wait until the screen is still, then save <out-dir>/<name>.json
//!   cd <dir>          folder for the next `start`
//!   run <command>     run a shell command (e.g. change the configuration)

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Session {
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    _master: Box<dyn MasterPty + Send>,
    screen: Arc<Mutex<(vt100::Parser, Instant)>>,
}

fn start(pairee: &str, cols: u16, rows: u16, cwd: &str, args: &[String]) -> Session {
    let pty = native_pty_system()
        .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .expect("openpty");
    let mut cmd = CommandBuilder::new(pairee);
    cmd.args(args);
    cmd.cwd(cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    let child = pty.slave.spawn_command(cmd).expect("spawn pairee");
    let mut reader = pty.master.try_clone_reader().expect("reader");
    let writer = pty.master.take_writer().expect("writer");
    let screen = Arc::new(Mutex::new((vt100::Parser::new(rows, cols, 0), Instant::now())));
    let shared = Arc::clone(&screen);
    // Answers the queries crossterm makes at start-up (cursor position,
    // device attributes), as a terminal would.
    let replies = Arc::new(Mutex::new(Vec::<u8>::new()));
    let pending = Arc::clone(&replies);
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            let chunk = &buf[..n];
            let mut guard = shared.lock().unwrap();
            guard.0.process(chunk);
            guard.1 = Instant::now();
            let (row, col) = guard.0.screen().cursor_position();
            drop(guard);
            let text = String::from_utf8_lossy(chunk);
            let mut out = pending.lock().unwrap();
            if text.contains("\x1b[6n") {
                out.extend(format!("\x1b[{};{}R", row + 1, col + 1).bytes());
            }
            if text.contains("\x1b[c") || text.contains("\x1b[0c") {
                out.extend(b"\x1b[?62;22c");
            }
        }
    });
    let mut session = Session { child, writer, _master: pty.master, screen };
    // Flush replies while the program starts.
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        let data: Vec<u8> = std::mem::take(&mut *replies.lock().unwrap());
        if !data.is_empty() {
            session.writer.write_all(&data).unwrap();
            session.writer.flush().unwrap();
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    settle(&session);
    session
}

/// Waits until nothing was drawn for a while (or 8 s passed).
fn settle(session: &Session) {
    let begin = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(100));
        let quiet = session.screen.lock().unwrap().1.elapsed();
        if quiet > Duration::from_millis(900) || begin.elapsed() > Duration::from_secs(8) {
            return;
        }
    }
}

fn key_bytes(token: &str) -> Vec<u8> {
    if let Some(text) = token.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
        return text.replace("\\s", " ").into_bytes();
    }
    let function = |n: &str| -> Option<&'static str> {
        Some(match n {
            "F1" => "P",
            "F2" => "Q",
            "F3" => "R",
            "F4" => "S",
            _ => return None,
        })
    };
    let tilde = |n: &str| -> Option<u8> {
        Some(match n {
            "F5" => 15,
            "F6" => 17,
            "F7" => 18,
            "F8" => 19,
            "F9" => 20,
            "F10" => 21,
            "F11" => 23,
            "F12" => 24,
            "Ins" => 2,
            "Del" => 3,
            "PgUp" => 5,
            "PgDn" => 6,
            _ => return None,
        })
    };
    // Modified function keys: xterm's `CSI 1;m X` / `CSI n;m ~`.
    for (prefix, m) in [("A-", 3), ("S-", 2), ("C-", 5), ("CS-", 6)] {
        if let Some(name) = token.strip_prefix(prefix) {
            if let Some(f) = function(name) {
                return format!("\x1b[1;{m}{f}").into_bytes();
            }
            if let Some(n) = tilde(name) {
                return format!("\x1b[{n};{m}~").into_bytes();
            }
            let arrows = [("Up", 'A'), ("Down", 'B'), ("Right", 'C'), ("Left", 'D'), ("Home", 'H'), ("End", 'F')];
            if let Some((_, c)) = arrows.iter().find(|(a, _)| *a == name) {
                return format!("\x1b[1;{m}{c}").into_bytes();
            }
        }
    }
    if let Some(f) = function(token) {
        return format!("\x1bO{f}").into_bytes();
    }
    if let Some(n) = tilde(token) {
        return format!("\x1b[{n}~").into_bytes();
    }
    match token {
        "Enter" => b"\r".to_vec(),
        "Esc" => b"\x1b".to_vec(),
        "Tab" => b"\t".to_vec(),
        "Bksp" => b"\x7f".to_vec(),
        "Space" => b" ".to_vec(),
        "Up" => b"\x1b[A".to_vec(),
        "Down" => b"\x1b[B".to_vec(),
        "Right" => b"\x1b[C".to_vec(),
        "Left" => b"\x1b[D".to_vec(),
        "Home" => b"\x1b[H".to_vec(),
        "End" => b"\x1b[F".to_vec(),
        _ => {
            if let Some(c) = token.strip_prefix("CA-").filter(|c| c.len() == 1) {
                return vec![0x1b, c.as_bytes()[0].to_ascii_lowercase() & 0x1f];
            }
            if let Some(c) = token.strip_prefix("C-").filter(|c| c.len() == 1) {
                return vec![c.as_bytes()[0].to_ascii_lowercase() & 0x1f];
            }
            if let Some(c) = token.strip_prefix("A-").filter(|c| c.chars().count() == 1) {
                let mut v = vec![0x1b];
                v.extend(c.as_bytes());
                return v;
            }
            panic!("unknown key {token}")
        }
    }
}

fn color(c: vt100::Color) -> serde_json::Value {
    match c {
        vt100::Color::Default => serde_json::Value::Null,
        vt100::Color::Idx(i) => serde_json::json!(i),
        vt100::Color::Rgb(r, g, b) => serde_json::json!([r, g, b]),
    }
}

fn dump(session: &Session, path: &str) {
    let guard = session.screen.lock().unwrap();
    let screen = guard.0.screen();
    let (rows, cols) = screen.size();
    let mut lines = Vec::new();
    for r in 0..rows {
        let mut cells = Vec::new();
        for c in 0..cols {
            let Some(cell) = screen.cell(r, c) else { continue };
            cells.push(serde_json::json!({
                "t": cell.contents(),
                "fg": color(cell.fgcolor()),
                "bg": color(cell.bgcolor()),
                "b": cell.bold(),
                "i": cell.italic(),
                "u": cell.underline(),
                "r": cell.inverse(),
                "w": cell.is_wide(),
                "wc": cell.is_wide_continuation(),
            }));
        }
        lines.push(serde_json::Value::Array(cells));
    }
    let doc = serde_json::json!({ "rows": rows, "cols": cols, "cells": lines });
    std::fs::write(path, serde_json::to_vec(&doc).unwrap()).expect("write shot");
    eprintln!("saved {path}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, pairee, cols, rows, scenario, out, ..] = args.as_slice() else {
        eprintln!("usage: capture <pairee> <cols> <rows> <scenario> <out-dir>");
        std::process::exit(2);
    };
    let (cols, rows): (u16, u16) = (cols.parse().unwrap(), rows.parse().unwrap());
    std::fs::create_dir_all(out).unwrap();
    let mut cwd = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    let mut session: Option<Session> = None;
    for line in std::fs::read_to_string(scenario).expect("scenario").lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        let words: Vec<String> = rest.split_whitespace().map(str::to_string).collect();
        match cmd {
            "cd" => cwd = rest.trim().to_string(),
            "start" => {
                if let Some(mut s) = session.take() {
                    let _ = s.child.kill();
                }
                session = Some(start(pairee, cols, rows, &cwd, &words));
            }
            "keys" => {
                let s = session.as_mut().expect("start first");
                for token in &words {
                    s.writer.write_all(&key_bytes(token)).unwrap();
                    s.writer.flush().unwrap();
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
            "run" => {
                let status = std::process::Command::new("sh").arg("-c").arg(rest).status();
                assert!(status.is_ok_and(|s| s.success()), "run failed: {rest}");
            }
            "wait" => std::thread::sleep(Duration::from_millis(rest.trim().parse().unwrap())),
            "shot" => {
                let s = session.as_ref().expect("start first");
                settle(s);
                dump(s, &format!("{out}/{}.json", rest.trim()));
            }
            other => panic!("unknown scenario command {other}"),
        }
    }
    if let Some(mut s) = session {
        let _ = s.child.kill();
    }
}
