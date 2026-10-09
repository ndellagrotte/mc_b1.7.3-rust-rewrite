//! Importing a vanilla save folder (PLAN_v4 D5, §3.5): the folder is read
//! into memory, checked by the simulation's import validation
//! ([`mc_sim::save::import::validate`]: the first block or item id Beta
//! 1.7.3 does not define refuses it, and nothing is copied), then written
//! into `saves/<name>` of the data directory through the native
//! [`Storage`], `<name>` being the folder's name made unused the way
//! `GuiCreateWorld.generateUnusedFolderName` does it (append `-`,
//! GuiCreateWorld.java:52-58).
//!
//! The refusal dialog is WP-3.6's; here a refusal is an [`ImportError`].

#![allow(clippy::disallowed_methods)]

use std::path::Path;

use mc_sim::save::import::{ImportRefusal, validate};
use mc_sim::save::{MemoryStorage, SaveConverterMcRegion, SaveFormat, SaveStore, Storage};

use crate::paths::SAVES_DIR;
use crate::storage::{FsStorage, WallClock};

/// Why an import failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    /// The folder could not be read.
    Read(String),
    /// D5: the save holds an id Beta 1.7.3 does not define (or cannot be
    /// read).
    Refused(ImportRefusal),
    /// Writing into the saves directory failed.
    Write(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Read(m) => write!(f, "cannot read the save folder: {m}"),
            ImportError::Refused(r) => write!(f, "the world cannot be imported: {r}"),
            ImportError::Write(m) => write!(f, "cannot write the imported world: {m}"),
        }
    }
}

impl std::error::Error for ImportError {}

/// The in-memory prefix the folder is staged under for the check.
const STAGING: &str = "import";

/// A refusal of the staged copy, naming the file in the real folder
/// (`<src>/level.dat`, not `import/level.dat`).
fn name_real_path(r: ImportRefusal, src: &Path) -> ImportRefusal {
    match r {
        ImportRefusal::Unreadable { path, reason } => {
            let path = match path.strip_prefix(STAGING) {
                Some(rest) => {
                    let rel = rest.trim_start_matches('/');
                    if rel.is_empty() {
                        src.display().to_string()
                    } else {
                        src.join(rel).display().to_string()
                    }
                }
                None => path,
            };
            ImportRefusal::Unreadable { path, reason }
        }
        other => other,
    }
}

impl ImportError {
    /// The error as the log names it (Gate 3 O4): a refusal of an
    /// unreadable file names it in the real folder `src` (an absolute path).
    /// The error itself keeps the staging-relative `import/<rel>`, which the
    /// refusal dialog shows as `<rel>` (an absolute path has no spaces to
    /// wrap at and would run past the dialog's right edge).
    pub fn naming_folder(&self, src: &Path) -> ImportError {
        match self {
            ImportError::Refused(r) => ImportError::Refused(name_real_path(r.clone(), src)),
            other => other.clone(),
        }
    }
}

/// The files under `root`, as `/`-separated paths relative to it, sorted.
fn read_tree(root: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let it = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in it {
            let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let r = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let ft = entry
                .file_type()
                .map_err(|e| format!("{}: {e}", path.display()))?;
            if ft.is_dir() {
                stack.push((path, r));
            } else if ft.is_file() {
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                out.push((r, bytes));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Imports the save folder `src` into `<data_dir>/saves` (module docs);
/// returns the world's directory name there.
pub fn import_world(data_dir: &Path, src: &Path) -> Result<String, ImportError> {
    let files = read_tree(src).map_err(ImportError::Read)?;
    if files.is_empty() {
        return Err(ImportError::Read(format!("{} is empty", src.display())));
    }
    // Validate a staged copy: nothing touches the saves directory unless
    // every id is defined.
    let mut staged = MemoryStorage::default();
    for (rel, bytes) in &files {
        staged
            .write(&format!("import/{rel}"), bytes)
            .map_err(|e| ImportError::Read(e.to_string()))?;
    }
    validate(&mut staged, STAGING).map_err(ImportError::Refused)?;

    let fs = FsStorage::new(data_dir).map_err(|e| ImportError::Write(e.to_string()))?;
    let store = SaveStore::shared(Box::new(fs.clone()), Box::new(WallClock));
    let mut loader = SaveConverterMcRegion::new(store, SAVES_DIR);
    let mut name = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty() && n != "." && n != "..")
        .unwrap_or_else(|| "World".to_string());
    let mut fs = fs;
    // generateUnusedFolderName, also skipping folders that exist without a
    // level.dat (the import must not merge into them).
    while loader.get_world_info(&name).is_some()
        || !fs
            .list(&format!("{SAVES_DIR}/{name}"))
            .unwrap_or_default()
            .is_empty()
    {
        name.push('-');
    }
    for (rel, bytes) in &files {
        fs.write(&format!("{SAVES_DIR}/{name}/{rel}"), bytes)
            .map_err(|e| ImportError::Write(e.to_string()))?;
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TempDir;

    fn level_dat(level_name: &str) -> Vec<u8> {
        use mc_sim::save::Tag;
        let info = mc_sim::world::info::WorldInfo::new(5, level_name);
        let data = info.get_nbt_tag_compound(0);
        let mut root = std::collections::BTreeMap::new();
        root.insert("Data".to_string(), Tag::Compound(data));
        mc_sim::save::mc_nbt::write_gzip("", &Tag::Compound(root)).unwrap()
    }

    #[test]
    fn imports_and_renames_on_collision() {
        let src = TempDir::new("import_src");
        let data = TempDir::new("import_data");
        let world = src.0.join("MyWorld");
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), level_dat("My World")).unwrap();
        let a = import_world(&data.0, &world).unwrap();
        assert_eq!(a, "MyWorld");
        assert!(data.0.join("saves/MyWorld/level.dat").exists());
        let b = import_world(&data.0, &world).unwrap();
        assert_eq!(b, "MyWorld-");
    }

    #[test]
    fn a_folder_without_level_dat_is_refused_and_nothing_is_copied() {
        let src = TempDir::new("import_bad");
        let data = TempDir::new("import_bad_data");
        let world = src.0.join("Bad");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(world.join("junk.txt"), b"x").unwrap();
        let e = import_world(&data.0, &world).unwrap_err();
        assert!(matches!(e, ImportError::Refused(_)), "{e}");
        assert!(!data.0.join("saves").exists());
        // The log names the real file, not the staging copy (Gate 3 O4)...
        let real = world.join("level.dat").display().to_string();
        let logged = e.naming_folder(&world).to_string();
        assert!(logged.contains(&real), "{logged}");
        assert!(!logged.contains("import/"), "{logged}");
        // ...and the dialog the short name, which it can wrap.
        let (_, lines) = crate::gui::menu::import_refused::refusal_text(&e);
        assert!(lines[0].starts_with("Cannot read level.dat:"), "{lines:?}");
    }

    #[test]
    fn an_unknown_item_id_is_refused_with_its_id_and_nothing_is_copied() {
        use mc_sim::save::{Tag, TagType};
        use std::collections::BTreeMap;
        let src = TempDir::new("import_d5");
        let data = TempDir::new("import_d5_data");
        let world = src.0.join("Modded");
        std::fs::create_dir_all(&world).unwrap();
        let mut stack = BTreeMap::new();
        stack.insert("id".to_string(), Tag::Short(4000));
        stack.insert("Count".to_string(), Tag::Byte(1));
        stack.insert("Damage".to_string(), Tag::Short(0));
        stack.insert("Slot".to_string(), Tag::Byte(3));
        let mut player = BTreeMap::new();
        player.insert(
            "Inventory".to_string(),
            Tag::List {
                elem: TagType::Compound,
                items: vec![Tag::Compound(stack)],
            },
        );
        let info = mc_sim::world::info::WorldInfo::new(5, "Modded");
        let data_c = info.get_nbt_tag_compound_with_player(Some(Tag::Compound(player)), 0);
        let mut root = BTreeMap::new();
        root.insert("Data".to_string(), Tag::Compound(data_c));
        let bytes = mc_sim::save::mc_nbt::write_gzip("", &Tag::Compound(root)).unwrap();
        std::fs::write(world.join("level.dat"), bytes).unwrap();
        let e = import_world(&data.0, &world).unwrap_err();
        let ImportError::Refused(r) = &e else {
            panic!("{e}");
        };
        assert!(
            matches!(r, ImportRefusal::UnknownItem { id: 4000, .. }),
            "{r:?}"
        );
        assert!(e.to_string().contains("4000"), "{e}");
        assert!(!data.0.join("saves").exists());
    }

    /// The reference testWorld when present (never committed, D8).
    #[test]
    fn the_reference_test_world_when_present() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.reference/mc_b1.7.3-world/testWorld");
        if !src.exists() {
            return;
        }
        let data = TempDir::new("import_ref");
        let name = import_world(&data.0, &src).unwrap();
        assert_eq!(name, "testWorld");
        assert!(data.0.join("saves/testWorld/level.dat").exists());
    }
}
