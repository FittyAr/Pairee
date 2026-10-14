//! Transfer reports (HTML or CSV) of a job's results, in the user's
//! language. File names come from anywhere (servers, archives), so every
//! value is escaped for the format it goes into.

use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::job::TransferResults;
use crate::config::localization::t;

/// Columns of both reports, as i18n keys.
const COLUMNS: [&str; 7] = [
    "report_col_status",
    "report_col_source",
    "report_col_destination",
    "report_col_size",
    "report_col_src_hash",
    "report_col_dst_hash",
    "report_col_info",
];

/// One row of a report, before formatting.
struct Row {
    class: &'static str,
    status: &'static str,
    cells: [String; 6],
}

/// The rows of `results`: failures, then skips, then completed files.
fn rows(results: &TransferResults) -> Vec<Row> {
    let failed = results.failed_files.iter().map(|f| Row {
        class: "error",
        status: "FAIL",
        cells: [
            f.src.to_string_lossy().into_owned(),
            f.dst.to_string_lossy().into_owned(),
            "-".into(),
            "-".into(),
            "-".into(),
            t("report_error_retries")
                .replacen("{}", &f.error, 1)
                .replacen("{}", &f.retries.to_string(), 1),
        ],
    });
    let skipped = results.skipped_files.iter().map(|f| Row {
        class: "warning",
        status: "SKIP",
        cells: [
            f.src.to_string_lossy().into_owned(),
            "-".into(),
            "-".into(),
            "-".into(),
            "-".into(),
            t("report_skipped_reason").replacen("{}", &f.reason, 1),
        ],
    });
    let completed = results.completed_files.iter().map(|f| Row {
        class: "ok",
        status: "OK",
        cells: [
            f.src.to_string_lossy().into_owned(),
            f.dst.to_string_lossy().into_owned(),
            bytesize::ByteSize(f.size).to_string(),
            f.src_hash.clone().unwrap_or_else(|| "-".into()),
            f.dst_hash.clone().unwrap_or_else(|| "-".into()),
            format!("{:.2?}", f.duration),
        ],
    });
    failed.chain(skipped).chain(completed).collect()
}

/// `text` safe inside HTML element content and quoted attributes.
fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// `text` as one quoted CSV field. A value that a spreadsheet would run as
/// a formula (`=`, `+`, `-`, `@`, tab, CR first) is prefixed with `'`.
fn csv_field(text: &str) -> String {
    let guarded = if text.starts_with(['=', '+', '-', '@', '\t', '\r']) && text != "-" {
        format!("'{text}")
    } else {
        text.to_string()
    };
    format!("\"{}\"", guarded.replace('"', "\"\""))
}

/// The HTML report of `results` for the job named `job_name`.
pub fn generate_html_report(results: &TransferResults, job_name: &str) -> String {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let job_name = html_escape(job_name);
    let symbol = |status| match status {
        "FAIL" => "✗",
        "SKIP" => "⚠",
        _ => "✓",
    };
    let body: String = rows(results)
        .iter()
        .map(|row| {
            let cells: String = row
                .cells
                .iter()
                .map(|cell| format!("<td>{}</td>", html_escape(cell)))
                .collect();
            format!(
                "<tr class='{}'><td>{}</td>{cells}</tr>\n",
                row.class,
                symbol(row.status)
            )
        })
        .collect();
    let header: String = COLUMNS
        .iter()
        .map(|key| format!("<th>{}</th>", html_escape(&t(key))))
        .collect();
    let title = html_escape(&t("report_title"));
    let summary = |key: &str| html_escape(&t(key));
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>{title} — {job_name}</title>
    <style>
        body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; margin: 20px; background-color: #f8f9fa; color: #212529; }}
        h1 {{ color: #0d6efd; border-bottom: 2px solid #dee2e6; padding-bottom: 10px; }}
        .summary {{ background: #fff; padding: 15px; border-radius: 5px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); margin-bottom: 20px; }}
        .summary p {{ margin: 5px 0; }}
        table {{ width: 100%; border-collapse: collapse; background: #fff; border-radius: 5px; overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #dee2e6; }}
        th {{ background-color: #0d6efd; color: #fff; }}
        tr:hover {{ background-color: #f1f3f5; }}
        .ok {{ color: #198754; }}
        .error {{ background-color: #f8d7da; color: #842029; }}
        .warning {{ background-color: #fff3cd; color: #664d03; }}
        .badge {{ padding: 4px 8px; border-radius: 4px; font-weight: bold; font-size: 0.9em; }}
        .badge-ok {{ background: #d1e7dd; color: #0f5132; }}
        .badge-err {{ background: #f8d7da; color: #842029; }}
    </style>
</head>
<body>
    <h1>{title}</h1>
    <div class="summary">
        <p><strong>{task}</strong> {job_name}</p>
        <p><strong>{generated}</strong> {now}</p>
        <p><strong>{ok}</strong> <span class="badge badge-ok">{}</span></p>
        <p><strong>{failed}</strong> <span class="badge badge-err">{}</span></p>
        <p><strong>{skipped}</strong> <span class="badge">{}</span></p>
    </div>
    <h2>{details}</h2>
    <table>
        <thead>
            <tr>{header}</tr>
        </thead>
        <tbody>
{body}        </tbody>
    </table>
</body>
</html>"#,
        results.completed_files.len(),
        results.failed_files.len(),
        results.skipped_files.len(),
        task = summary("report_task"),
        generated = summary("report_generated"),
        ok = summary("report_completed_count"),
        failed = summary("report_failed_count"),
        skipped = summary("report_skipped_count"),
        details = summary("report_details"),
    )
}

/// The CSV report of `results` (UTF-8 with a BOM, for spreadsheets).
pub fn generate_csv_report(results: &TransferResults) -> String {
    let mut csv = String::from('\u{FEFF}');
    let header: Vec<String> = COLUMNS.iter().map(|key| csv_field(&t(key))).collect();
    csv.push_str(&header.join(","));
    csv.push('\n');
    for row in rows(results) {
        let fields: Vec<String> = std::iter::once(row.status.to_string())
            .chain(row.cells.iter().map(|cell| csv_field(cell)))
            .collect();
        csv.push_str(&fields.join(","));
        csv.push('\n');
    }
    csv
}

/// Saves the report in `destination_dir` under a new name (an existing
/// file is never replaced) and returns its path.
pub fn save_report(
    report_content: &str,
    format: &str,
    destination_dir: &Path,
) -> std::io::Result<PathBuf> {
    let _ = std::fs::create_dir_all(destination_dir);

    let now = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let ext = match format.to_lowercase().as_str() {
        "csv" => "csv",
        _ => "html",
    };

    let mut attempt = 0u32;
    loop {
        let suffix = match attempt {
            0 => String::new(),
            n => format!("_{n}"),
        };
        let path = destination_dir.join(format!("transfer_report_{now}{suffix}.{ext}"));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(report_content.as_bytes())?;
                file.sync_all()?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && attempt < 1000 => {
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::job::{FailedFile, FileTransferResult};
    use std::time::Duration;

    fn results() -> TransferResults {
        TransferResults {
            completed_files: vec![FileTransferResult {
                src: PathBuf::from("<img src=x onerror=alert(1)>.txt"),
                dst: PathBuf::from("=HYPERLINK(\"http://x\").txt"),
                size: 3,
                src_hash: None,
                dst_hash: None,
                verified: false,
                duration: Duration::from_millis(5),
                replaced: Default::default(),
                dst_stamp: None,
            }],
            failed_files: vec![FailedFile {
                src: PathBuf::from("a&b.txt"),
                dst: PathBuf::new(),
                error: "boom".into(),
                retries: 2,
            }],
            ..TransferResults::default()
        }
    }

    #[test]
    fn html_reports_escape_names() {
        let html = generate_html_report(&results(), "<job>");
        assert!(!html.contains("<img"), "{html}");
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;.txt"));
        assert!(html.contains("a&amp;b.txt"));
        assert!(html.contains("&lt;job&gt;"));
    }

    #[test]
    fn csv_reports_never_start_a_formula() {
        let csv = generate_csv_report(&results());
        assert!(
            csv.contains("\"'=HYPERLINK(\"\"http://x\"\").txt\""),
            "{csv}"
        );
        assert!(!csv.contains(",\"=HYPERLINK"), "{csv}");
    }

    #[test]
    fn reports_never_replace_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let first = save_report("one", "csv", dir.path()).unwrap();
        let second = save_report("two", "csv", dir.path()).unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read_to_string(first).unwrap(), "one");
        assert_eq!(std::fs::read_to_string(second).unwrap(), "two");
    }
}
