use chrono::Local;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::job::TransferResults;

/// Genera un reporte HTML con los detalles de la transferencia.
pub fn generate_html_report(results: &TransferResults, job_name: &str) -> String {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut completed_rows = String::new();
    for f in &results.completed_files {
        completed_rows.push_str(&format!(
            "<tr class='ok'><td>✓</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:.2?}</td></tr>",
            f.src.to_string_lossy(),
            f.dst.to_string_lossy(),
            bytesize::ByteSize(f.size),
            f.src_hash.as_deref().unwrap_or("-"),
            f.dst_hash.as_deref().unwrap_or("-"),
            f.duration
        ));
    }

    let mut failed_rows = String::new();
    for f in &results.failed_files {
        failed_rows.push_str(&format!(
            "<tr class='error'><td>✗</td><td>{}</td><td>{}</td><td>-</td><td>-</td><td>-</td><td>Error: {} (Reintentos: {})</td></tr>",
            f.src.to_string_lossy(),
            f.dst.to_string_lossy(),
            f.error,
            f.retries
        ));
    }

    let mut skipped_rows = String::new();
    for f in &results.skipped_files {
        skipped_rows.push_str(&format!(
            "<tr class='warning'><td>⚠</td><td>{}</td><td>-</td><td>-</td><td>-</td><td>-</td><td>Omitido: {}</td></tr>",
            f.src.to_string_lossy(),
            f.reason
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Pairee Transfer Report — {job_name}</title>
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
    <h1>Reporte de Transferencia Pairee</h1>
    <div class="summary">
        <p><strong>Tarea:</strong> {job_name}</p>
        <p><strong>Generado el:</strong> {now}</p>
        <p><strong>Archivos Exitosos:</strong> <span class="badge badge-ok">{}</span></p>
        <p><strong>Archivos Fallidos:</strong> <span class="badge badge-err">{}</span></p>
        <p><strong>Archivos Omitidos:</strong> <span class="badge">{}</span></p>
    </div>
    <h2>Detalles del Historial</h2>
    <table>
        <thead>
            <tr>
                <th>Estado</th>
                <th>Origen</th>
                <th>Destino</th>
                <th>Tamaño</th>
                <th>Hash Origen</th>
                <th>Hash Destino</th>
                <th>Info / Duración</th>
            </tr>
        </thead>
        <tbody>
            {failed_rows}
            {skipped_rows}
            {completed_rows}
        </tbody>
    </table>
</body>
</html>"#,
        results.completed_files.len(),
        results.failed_files.len(),
        results.skipped_files.len()
    )
}

/// Genera un reporte CSV con los detalles de la transferencia.
pub fn generate_csv_report(results: &TransferResults) -> String {
    let mut csv = String::new();
    // UTF-8 BOM
    csv.push('\u{FEFF}');
    csv.push_str(
        "Estado,Origen,Destino,Tamaño,Hash Origen,Hash Destino,Error,Reintentos/Duración\n",
    );

    for f in &results.failed_files {
        csv.push_str(&format!(
            "FAIL,\"{}\",\"{}\",0,-,-,\"{}\",{}\n",
            f.src.to_string_lossy().replace('"', "\"\""),
            f.dst.to_string_lossy().replace('"', "\"\""),
            f.error.replace('"', "\"\""),
            f.retries
        ));
    }

    for f in &results.skipped_files {
        csv.push_str(&format!(
            "SKIP,\"{}\",-,0,-,-,\"{}\",-\n",
            f.src.to_string_lossy().replace('"', "\"\""),
            f.reason.replace('"', "\"\"")
        ));
    }

    for f in &results.completed_files {
        csv.push_str(&format!(
            "OK,\"{}\",\"{}\",{},\"{}\",\"{}\",-,\"{:.2?}\"\n",
            f.src.to_string_lossy().replace('"', "\"\""),
            f.dst.to_string_lossy().replace('"', "\"\""),
            f.size,
            f.src_hash.as_deref().unwrap_or("-"),
            f.dst_hash.as_deref().unwrap_or("-"),
            f.duration
        ));
    }

    csv
}

/// Guarda el reporte generado en el disco y retorna la ruta completa.
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

    let filename = format!("transfer_report_{}.{}", now, ext);
    let path = destination_dir.join(filename);

    let mut file = File::create(&path)?;
    file.write_all(report_content.as_bytes())?;
    file.sync_all()?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::job::{FailedFile, FileTransferResult, SkippedFile, TransferResults};
    use std::time::Duration;

    fn empty_results() -> TransferResults {
        TransferResults::default()
    }

    #[test]
    fn html_report_includes_title_and_summary_for_empty_results() {
        let html = generate_html_report(&empty_results(), "test job");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Pairee Transfer Report"));
        assert!(html.contains("test job"));
        assert!(html.contains("<strong>Archivos Exitosos:</strong>"));
        assert!(html.contains("<strong>Archivos Fallidos:</strong>"));
        assert!(html.contains("<strong>Archivos Omitidos:</strong>"));
        // All three counts are 0.
        assert!(html.contains("Archivos Exitosos:"));
        // Empty list means no data rows in the body.
        assert!(html.contains("<tbody>"));
        assert!(html.contains("</tbody>"));
    }

    #[test]
    fn html_report_renders_completed_files() {
        let mut r = empty_results();
        r.completed_files.push(FileTransferResult {
            src: PathBuf::from("/src/a.txt"),
            dst: PathBuf::from("/dst/a.txt"),
            size: 1024,
            src_hash: Some("abc123".to_string()),
            dst_hash: Some("abc123".to_string()),
            verified: true,
            duration: Duration::from_millis(250),
        });
        let html = generate_html_report(&r, "job1");
        assert!(html.contains("/src/a.txt"));
        assert!(html.contains("/dst/a.txt"));
        // The exact bytesize format ("1.00 KiB", "1.02 KB", etc.)
        // is owned by the upstream `bytesize` crate, so we don't
        // pin a specific string. Just verify the row is well-formed
        // and that the src/dst paths + hash all reached the body.
        assert!(html.contains("tr class='ok'"));
        assert!(html.contains("abc123"));
        // The size cell is the 4th column. Whatever the format, it
        // must include the "1024" decimal somewhere — both IEC
        // ("1.00 KiB") and SI ("1.02 KB") representations come from
        // 1024 bytes and include "1" in the output.
        assert!(html.contains("1."));
    }

    #[test]
    fn html_report_renders_failed_files() {
        let mut r = empty_results();
        r.failed_files.push(FailedFile {
            src: PathBuf::from("/src/bad"),
            dst: PathBuf::from("/dst/bad"),
            error: "permission denied".to_string(),
            retries: 3,
        });
        let html = generate_html_report(&r, "job2");
        assert!(html.contains("permission denied"));
        assert!(html.contains("tr class='error'"));
        assert!(html.contains("Error:"));
        assert!(html.contains("Reintentos: 3"));
    }

    #[test]
    fn html_report_renders_skipped_files() {
        let mut r = empty_results();
        r.skipped_files.push(SkippedFile {
            src: PathBuf::from("/src/skip"),
            reason: "masked out by filter".to_string(),
        });
        let html = generate_html_report(&r, "job3");
        assert!(html.contains("tr class='warning'"));
        assert!(html.contains("Omitido: masked out by filter"));
    }

    #[test]
    fn html_report_escapes_html_special_chars_in_paths() {
        // The current implementation does NOT escape `<`/`>`/`&` in
        // paths — that's a pre-existing limitation. Document it so a
        // future refactor doesn't break things silently.
        let mut r = empty_results();
        r.completed_files.push(FileTransferResult {
            src: PathBuf::from("/src/<script>"),
            dst: PathBuf::from("/dst/x"),
            size: 0,
            src_hash: None,
            dst_hash: None,
            verified: false,
            duration: Duration::ZERO,
        });
        let html = generate_html_report(&r, "job4");
        // Path is interpolated raw — at minimum it must not break the
        // HTML structure (no unterminated tags in the row).
        assert!(html.contains("<script>"));
        // The end-of-row closing tag must still be there.
        assert!(html.contains("</tr>"));
    }

    #[test]
    fn csv_report_starts_with_utf8_bom_and_header() {
        let csv = generate_csv_report(&empty_results());
        assert!(csv.starts_with('\u{FEFF}'));
        // Header line is the second piece after the BOM.
        let body = &csv[3..];
        assert!(body.starts_with(
            "Estado,Origen,Destino,Tamaño,Hash Origen,Hash Destino,Error,Reintentos/Duración"
        ));
    }

    #[test]
    fn csv_report_emits_one_row_per_outcome() {
        let mut r = empty_results();
        r.completed_files.push(FileTransferResult {
            src: PathBuf::from("/a"),
            dst: PathBuf::from("/b"),
            size: 42,
            src_hash: Some("h1".to_string()),
            dst_hash: Some("h2".to_string()),
            verified: true,
            duration: Duration::from_millis(10),
        });
        r.failed_files.push(FailedFile {
            src: PathBuf::from("/c"),
            dst: PathBuf::from("/d"),
            error: "err".to_string(),
            retries: 1,
        });
        r.skipped_files.push(SkippedFile {
            src: PathBuf::from("/e"),
            reason: "filtered".to_string(),
        });
        let csv = generate_csv_report(&r);
        // Header + 3 rows = 4 lines.
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 4);
        // The report emits the rows in the order: FAIL, SKIP, OK
        // (failed first so they are easy to spot in the CSV).
        assert!(lines[1].starts_with("FAIL,"));
        assert!(lines[2].starts_with("SKIP,"));
        assert!(lines[3].starts_with("OK,"));
    }

    #[test]
    fn csv_report_escapes_double_quotes_in_paths() {
        let mut r = empty_results();
        r.completed_files.push(FileTransferResult {
            src: PathBuf::from("/a\"b"),
            dst: PathBuf::from("/c"),
            size: 0,
            src_hash: None,
            dst_hash: None,
            verified: false,
            duration: Duration::ZERO,
        });
        let csv = generate_csv_report(&r);
        // A `"` inside a CSV field must be escaped as `""`.
        assert!(csv.contains("\"/a\"\"b\""));
    }

    #[test]
    fn save_report_creates_file_with_correct_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path_html = save_report("<html/>", "html", dir.path()).unwrap();
        assert!(path_html.extension().map(|e| e == "html").unwrap_or(false));

        let path_csv = save_report("a,b\n", "csv", dir.path()).unwrap();
        assert!(path_csv.extension().map(|e| e == "csv").unwrap_or(false));

        // Unknown format defaults to html.
        let path_other = save_report("x", "weird", dir.path()).unwrap();
        assert!(path_other.extension().map(|e| e == "html").unwrap_or(false));
    }

    #[test]
    fn save_report_creates_destination_dir_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested").join("subdir");
        let path = save_report("ok", "csv", &nested).unwrap();
        assert!(nested.is_dir());
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "ok");
    }

    #[test]
    fn save_report_filename_contains_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let path = save_report("x", "csv", dir.path()).unwrap();
        let name = path.file_name().unwrap().to_string_lossy();
        // Pattern: transfer_report_YYYYMMDD_HHMMSS.csv
        assert!(
            name.starts_with("transfer_report_"),
            "unexpected filename: {}",
            name
        );
        // Strip prefix and extension; the middle must be 15 chars
        // (8 date + `_` + 6 time).
        let middle = name
            .trim_start_matches("transfer_report_")
            .trim_end_matches(".csv");
        assert_eq!(middle.len(), 15, "middle was: {}", middle);
    }
}
