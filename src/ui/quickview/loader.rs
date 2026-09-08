use crate::config::localization::t;

pub fn load_quick_view_content(path: &std::path::Path) -> Vec<String> {
    if path.is_dir() {
        let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
        let mut lines = vec![
            t("quickview_folder").replacen("{}", &dir_name, 1),
            "────────────────────────────────────────".to_string(),
        ];
        if let Ok(entries) = std::fs::read_dir(path) {
            let mut entries_vec: Vec<_> = entries.flatten().collect();
            entries_vec.sort_by(|a, b| {
                let a_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let b_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
                match (a_dir, b_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        let a_name = a.file_name().to_string_lossy().to_lowercase();
                        let b_name = b.file_name().to_string_lossy().to_lowercase();
                        a_name.cmp(&b_name)
                    }
                }
            });
            for entry in entries_vec {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir {
                    lines.push(format!("{}/", name));
                } else {
                    lines.push(name);
                }
            }
        }
        return lines;
    }

    let format = crate::fs::archive::detect_format(path);
    match format {
        crate::fs::archive::ArchiveFormat::Zip
        | crate::fs::archive::ArchiveFormat::TarGz
        | crate::fs::archive::ArchiveFormat::SevenZ => {
            match crate::fs::archive::list_archive_files(path) {
                Ok(files) => {
                    let format_name = match format {
                        crate::fs::archive::ArchiveFormat::Zip => "ZIP",
                        crate::fs::archive::ArchiveFormat::TarGz => "TarGz",
                        crate::fs::archive::ArchiveFormat::SevenZ => "7Z",
                        _ => "Archive",
                    };
                    let archive_name = path.file_name().unwrap_or_default().to_string_lossy();
                    let files_count = files.len().to_string();
                    let mut lines = vec![
                        t("quickview_archive").replacen("{}", &archive_name, 1),
                        t("quickview_format").replacen("{}", format_name, 1),
                        t("quickview_files").replacen("{}", &files_count, 1),
                        "────────────────────────────────────────".to_string(),
                    ];
                    for f in files {
                        lines.push(f);
                    }
                    lines
                }
                Err(e) => {
                    let err_str = e.to_string();
                    vec![t("quickview_error").replacen("{}", &err_str, 1)]
                }
            }
        }
        _ => match std::fs::read_to_string(path) {
            Ok(text) => text.lines().map(|l| l.to_string()).collect(),
            Err(_) => vec![t("quickview_binary_no_preview")],
        },
    }
}
