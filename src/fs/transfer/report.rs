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
