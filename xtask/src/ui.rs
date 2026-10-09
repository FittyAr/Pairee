//! Console output and prompts.

use std::io::{self, BufRead, Write};

use crate::Result;

pub fn step(message: &str) {
    println!("==> {message}");
}

pub fn ok(message: &str) {
    println!("  ok: {message}");
}

pub fn warn(message: &str) {
    eprintln!("warning: {message}");
}

pub fn error(message: &str) {
    eprintln!("error: {message}");
}

/// Reads one answer; an empty line or end of input gives `default`.
pub fn ask(question: &str, default: &str) -> Result<String> {
    print!("{question} [{default}]: ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let answer = line.trim();
    Ok(if answer.is_empty() { default } else { answer }.to_string())
}

/// A yes/no question that defaults to "no"; `assume_yes` answers it unasked.
pub fn confirm(question: &str, assume_yes: bool) -> Result<bool> {
    if assume_yes {
        println!("{question} (y/n): y");
        return Ok(true);
    }
    Ok(ask(&format!("{question} (y/n)"), "n")?.eq_ignore_ascii_case("y"))
}
