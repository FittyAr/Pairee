//! Contracts for wipe/compress/extract style jobs (filesystem only).

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

#[test]
fn zip_roundtrip_preserves_file_content() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src");
    let out_dir = tmp.path().join("out");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&out_dir).unwrap();

    let payload = b"hello-transfer-ops";
    fs::write(src_dir.join("note.txt"), payload).unwrap();

    let archive = tmp.path().join("pack.zip");
    write_simple_zip(&archive, &src_dir.join("note.txt"), "note.txt").unwrap();
    assert!(archive.exists());
    assert!(archive.metadata().unwrap().len() > 0);

    extract_simple_zip(&archive, &out_dir).expect("extract zip");
    assert_eq!(
        fs::read(out_dir.join("note.txt")).expect("read extracted note"),
        payload
    );
}

#[test]
fn zip_extract_roundtrip_restores_file_content() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path().join("payload.bin");
    let payload = b"zip-extract-contract";
    fs::write(&src, payload).unwrap();

    let archive = tmp.path().join("roundtrip.zip");
    write_simple_zip(&archive, &src, "payload.bin").unwrap();

    let out_dir = tmp.path().join("extracted");
    fs::create_dir_all(&out_dir).unwrap();
    extract_simple_zip(&archive, &out_dir).unwrap();
    assert_eq!(fs::read(out_dir.join("payload.bin")).unwrap(), payload);
}

fn extract_simple_zip(archive: &Path, dest: &Path) -> std::io::Result<()> {
    let f = fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(f).map_err(std::io::Error::other)?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(std::io::Error::other)?;
        let name = file.name().to_string();
        if name.ends_with('/') || name.ends_with('\\') {
            continue;
        }
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        fs::write(dest.join(name), buf)?;
    }
    Ok(())
}

fn write_simple_zip(archive: &Path, file: &Path, name_in_zip: &str) -> std::io::Result<()> {
    let f = fs::File::create(archive)?;
    let mut zip = zip::ZipWriter::new(f);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(name_in_zip, options)
        .map_err(std::io::Error::other)?;
    let data = fs::read(file)?;
    zip.write_all(&data)?;
    zip.finish().map_err(std::io::Error::other)?;
    Ok(())
}
