use super::extract_zip;
use std::{fs, io::Write, path::Path};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

fn write_zip(path: &Path, name: &str) {
    let mut archive = zip::ZipWriter::new(fs::File::create(path).unwrap());
    archive
        .start_file(name, SimpleFileOptions::default())
        .unwrap();
    archive.write_all(b"plugin content").unwrap();
    archive.finish().unwrap();
}

#[test]
fn extracts_regular_zip_entry() {
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("plugin.zip");
    let destination = temp.path().join("output");
    write_zip(&archive, "plugin/data.txt");
    extract_zip(&archive, &destination).unwrap();
    assert_eq!(
        fs::read(destination.join("plugin/data.txt")).unwrap(),
        b"plugin content"
    );
}

#[test]
fn rejects_zip_path_traversal_without_writing_outside_destination() {
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("plugin.zip");
    let destination = temp.path().join("output");
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, b"preserve").unwrap();
    write_zip(&archive, "../outside.txt");
    let error = extract_zip(&archive, &destination).unwrap_err();
    assert!(error.to_string().contains("unsafe path"), "{error:#}");
    assert_eq!(fs::read(outside).unwrap(), b"preserve");
}

#[test]
fn rejects_nul_zip_name_without_extracting() {
    let temp = TempDir::new().unwrap();
    let archive = temp.path().join("plugin.zip");
    let destination = temp.path().join("output");
    let name = "evil.txt";
    write_zip(&archive, name);
    let mut bytes = fs::read(&archive).unwrap();
    let offsets: Vec<_> = bytes
        .windows(name.len())
        .enumerate()
        .filter_map(|(offset, candidate)| (candidate == name.as_bytes()).then_some(offset))
        .collect();
    assert_eq!(
        offsets.len(),
        2,
        "local and central names must both be changed"
    );
    for offset in offsets {
        bytes[offset] = 0;
    }
    fs::write(&archive, bytes).unwrap();
    let mut parsed = zip::ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
    assert!(parsed.by_index(0).unwrap().enclosed_name().is_none());
    let error = extract_zip(&archive, &destination).unwrap_err();
    assert!(error.to_string().contains("unsafe path"), "{error:#}");
    assert!(!destination.exists());
}
