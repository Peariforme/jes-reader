//! Reading the files given on the command line.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use jes_reader_core::{Dd, DdKind, parse_document, parse_file};

/// A DD read from a file (a whole spool gives several, with the same path).
pub struct File {
    pub path: PathBuf,
    pub dd: Dd,
}

/// Reads and parses every file under `paths`, sorted by path. Steps get
/// their program from the JESJCL of the same job: one of the files, or
/// `JESJCL.txt` next to the file.
pub fn load(paths: &[PathBuf]) -> Result<Vec<File>> {
    let mut found = Vec::new();
    for path in paths {
        collect(path, &mut found)?;
    }
    found.sort();
    found.dedup();

    let mut files = Vec::new();
    for path in found {
        for dd in parse_document(&path.to_string_lossy(), &text(&path)?) {
            files.push(File {
                path: path.clone(),
                dd,
            });
        }
    }

    let jcls: Vec<Dd> = files
        .iter()
        .filter(|f| f.dd.kind == DdKind::JesJcl)
        .map(|f| f.dd.clone())
        .collect();
    for file in &mut files {
        if file.dd.kind == DdKind::JesJcl {
            continue;
        }
        if let Some(jcl) = jcls.iter().find(|jcl| jcl.is_jcl_of(&file.dd)) {
            file.dd.attach_programs(jcl);
        } else if let Some(sibling) = file.path.parent().map(|p| p.join("JESJCL.txt"))
            && sibling.is_file()
            && let Ok(text) = text(&sibling)
        {
            file.dd
                .attach_programs(&parse_file(&sibling.to_string_lossy(), &text));
        }
    }
    Ok(files)
}

fn text(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    // A sysout may contain bytes badly converted from EBCDIC: tolerate them.
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// A file, or every file in a directory and its subdirectories.
fn collect(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_dir() {
        let entries =
            std::fs::read_dir(path).with_context(|| format!("reading {}", path.display()))?;
        for entry in entries {
            collect(&entry?.path(), files)?;
        }
    } else {
        files.push(path.to_owned());
    }
    Ok(())
}
