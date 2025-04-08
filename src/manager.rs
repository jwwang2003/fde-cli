use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use serde::{Serialize, Deserialize};

/// Represents a subfolder entry that has the three required files.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEntry {
    /// The name of the subfolder.
    pub folder: String,
    /// The full path of the subfolder.
    pub folder_path: PathBuf,
    /// Path to the dc_bit file.
    pub dc_bit: PathBuf,
    /// Path to the cons file.
    pub cons: PathBuf,
    /// Path to the meta file.
    pub meta: PathBuf,
}

/// Top-level scan result containing separate lists for projects and recipes.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub projects: Vec<FileEntry>,
    pub recipes: Vec<FileEntry>,
}

/// Scans the current working directory for the "projects" and "recipes" folders.
/// Only subfolders that contain all three required files are included.
pub fn scan() -> ScanResult {
    ScanResult {
        projects: scan_dir("projects"),
        recipes: scan_dir("recipes"),
    }
}

/// Scans a given directory (either "projects" or "recipes") for subdirectories
/// that contain all three matching files.
fn scan_dir(base: &str) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    let base_path = Path::new(base);

    if base_path.exists() && base_path.is_dir() {
        if let Ok(dir_entries) = fs::read_dir(base_path) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(folder_name) = path.file_name().and_then(|s| s.to_str()) {
                        if let Some(file_entry) = scan_folder(&path, folder_name) {
                            entries.push(file_entry);
                        }
                    }
                }
            }
        }
    }
    entries
}

/// Scans a specific subfolder for files matching the required patterns.
/// Returns a `FileEntry` if all three files are found; otherwise returns `None`.
fn scan_folder(folder_path: &Path, folder_name: &str) -> Option<FileEntry> {
    // Build regex patterns dynamically based on the folder name.
    let regex_dc = Regex::new(&format!(r"^{}_dc_bit\.bit$", folder_name)).ok()?;
    let regex_cons = Regex::new(&format!(r"^{}_cons\.xml$", folder_name)).ok()?;
    let regex_meta = Regex::new(&format!(r"^{}_meta\.json$", folder_name)).ok()?;

    let mut dc_bit: Option<PathBuf> = None;
    let mut cons: Option<PathBuf> = None;
    let mut meta: Option<PathBuf> = None;

    if let Ok(files) = fs::read_dir(folder_path) {
        for file in files.flatten() {
            let file_name = file.file_name();
            let file_name_str = file_name.to_string_lossy();

            if regex_dc.is_match(&file_name_str) {
                dc_bit = Some(file.path());
            } else if regex_cons.is_match(&file_name_str) {
                cons = Some(file.path());
            } else if regex_meta.is_match(&file_name_str) {
                meta = Some(file.path());
            }
        }
    }

    // Only return an entry if all three files are found.
    if let (Some(dc_bit), Some(cons), Some(meta)) = (dc_bit, cons, meta) {
        Some(FileEntry {
            folder: folder_name.to_string(),
            folder_path: folder_path.to_path_buf(),
            dc_bit,
            cons,
            meta,
        })
    } else {
        None
    }
}

/// Finds a FileEntry from a slice based on the folder name.
/// Returns a reference to the matching FileEntry if found.
pub fn find_file_entry_by_folder<'a>(entries: &'a [FileEntry], folder_name: &str) -> Option<&'a FileEntry> {
    entries.iter().find(|entry| entry.folder == folder_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan() {
        let result = scan();
        println!("{:#?}", result);
    }
}
