//! Asset access (D8, PLAN_v4 §3.5): the user's `minecraft.jar`, read as a
//! zip at start-up ([`JarAssets`]), and the `resources/` directory of sounds
//! and music ([`ResourcesDir`]). Nothing of Mojang's is embedded or
//! committed; a missing jar or directory is a start-up error naming the path
//! ([`AssetError`]).
//!
//! Resource names are the vanilla ones, absolute in the jar
//! (`TexturePackBase.getResourceAsStream("/terrain.png")`,
//! TexturePackBase.java:27-29: the default pack reads the client jar's own
//! resources). Texture packs (`TexturePackCustom`, `TexturePackList`) are
//! deferred (D3); the default pack's thumbnail and name are only its
//! `/pack.png` and `"Default"` (TexturePackDefault.java:7-13).
//!
//! The jar's resources are indexed into memory once (≈1.5 MB without the
//! classes), so readers need no file handle and the resource is `Send +
//! Sync`.

#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::Resource;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::lang::{EN_US_LANG, STATS_US_LANG, StringTranslate};

/// The terrain atlas (`RenderEngine.getTexture("/terrain.png")`,
/// Minecraft.java:967).
pub const TERRAIN_PNG: &str = "/terrain.png";
/// The font (Minecraft.java:241).
pub const FONT_DEFAULT_PNG: &str = "/font/default.png";
/// The default texture pack's thumbnail (TexturePackDefault.java:11).
pub const PACK_PNG: &str = "/pack.png";
/// The loading screen's background (LoadingScreenRenderer.java:145,
/// GuiScreen.java:151).
pub const GUI_BACKGROUND_PNG: &str = "/gui/background.png";
/// The missing-thumbnail texture (TexturePackDefault.java:28).
pub const GUI_UNKNOWN_PACK_PNG: &str = "/gui/unknown_pack.png";
/// The splash logo drawn by `loadScreen` (Minecraft.java:319).
pub const TITLE_MOJANG_PNG: &str = "/title/mojang.png";
/// The colour maps (Minecraft.java:242-244).
pub const MISC_WATERCOLOR_PNG: &str = "/misc/watercolor.png";
/// See [`MISC_WATERCOLOR_PNG`].
pub const MISC_GRASSCOLOR_PNG: &str = "/misc/grasscolor.png";
/// See [`MISC_WATERCOLOR_PNG`].
pub const MISC_FOLIAGECOLOR_PNG: &str = "/misc/foliagecolor.png";
/// The achievement GUID map (`AchievementMap`, AchievementMap.java).
pub const ACHIEVEMENT_MAP_TXT: &str = "/achievement/map.txt";

/// A start-up asset problem (D8: names the expected path).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetError {
    /// The jar is missing or unreadable.
    Jar {
        /// The path tried.
        path: PathBuf,
        /// Why.
        reason: String,
    },
    /// The `resources/` directory is missing.
    Resources {
        /// The path tried.
        path: PathBuf,
    },
    /// A resource is not in the jar.
    Missing {
        /// The resource name.
        name: String,
        /// The jar.
        jar: PathBuf,
    },
    /// A resource did not decode.
    Decode {
        /// The resource name.
        name: String,
        /// Why.
        reason: String,
    },
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::Jar { path, reason } => write!(
                f,
                "cannot read minecraft.jar at {} ({reason}); copy your own Beta 1.7.3 client jar there, or pass it with --jar <path> (docs/RUNNING.md)",
                path.display()
            ),
            AssetError::Resources { path } => write!(
                f,
                "no resources/ directory at {}; copy the directory with the sound/, music/, newsound/... folders there, or pass it with --resources <path> (docs/RUNNING.md)",
                path.display()
            ),
            AssetError::Missing { name, jar } => {
                write!(f, "{name} is not in {}", jar.display())
            }
            AssetError::Decode { name, reason } => write!(f, "cannot decode {name}: {reason}"),
        }
    }
}

impl std::error::Error for AssetError {}

/// The jar's resources (module docs).
#[derive(Resource, Clone, Debug)]
pub struct JarAssets {
    path: PathBuf,
    /// Every non-class, non-directory entry, by name without a leading `/`.
    entries: BTreeMap<String, Vec<u8>>,
}

impl JarAssets {
    /// Opens and indexes the jar at `path`.
    pub fn open(path: &Path) -> Result<Self, AssetError> {
        let err = |reason: String| AssetError::Jar {
            path: path.to_path_buf(),
            reason,
        };
        let bytes = std::fs::read(path).map_err(|e| err(e.to_string()))?;
        Self::from_bytes(path, bytes).map_err(err)
    }

    /// Indexes a jar already in memory (`path` is for messages).
    pub fn from_bytes(path: &Path, bytes: Vec<u8>) -> Result<Self, String> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        let mut entries = BTreeMap::new();
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
            if f.is_dir() || f.name().ends_with(".class") {
                continue;
            }
            let name = f.name().trim_start_matches('/').to_string();
            let mut buf = Vec::with_capacity(f.size() as usize);
            f.read_to_end(&mut buf)
                .map_err(|e| format!("{name}: {e}"))?;
            entries.insert(name, buf);
        }
        if !entries.contains_key(TERRAIN_PNG.trim_start_matches('/')) {
            return Err("no terrain.png in the jar (is it a Beta 1.7.3 client jar?)".into());
        }
        Ok(JarAssets {
            path: path.to_path_buf(),
            entries,
        })
    }

    /// The jar's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `getResourceAsStream(name)`'s bytes (`name` with or without the
    /// leading `/`), `None` when absent (Java: null).
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .get(name.trim_start_matches('/'))
            .map(|v| v.as_slice())
    }

    /// The bytes, or [`AssetError::Missing`].
    pub fn require(&self, name: &str) -> Result<&[u8], AssetError> {
        self.get(name).ok_or_else(|| AssetError::Missing {
            name: name.to_string(),
            jar: self.path.clone(),
        })
    }

    /// The names of the indexed resources (with a leading `/`).
    pub fn names(&self) -> impl Iterator<Item = String> + '_ {
        self.entries.keys().map(|k| format!("/{k}"))
    }

    /// A PNG as 8-bit RGBA (`ImageIO.read` then `getRGB`'s ARGB, in RGBA
    /// byte order), `(width, height, pixels)`.
    pub fn rgba(&self, name: &str) -> Result<(u32, u32, Vec<u8>), AssetError> {
        let bytes = self.require(name)?;
        let img =
            image::load_from_memory_with_format(bytes, image::ImageFormat::Png).map_err(|e| {
                AssetError::Decode {
                    name: name.to_string(),
                    reason: e.to_string(),
                }
            })?;
        let img = img.to_rgba8();
        let (w, h) = img.dimensions();
        Ok((w, h, img.into_raw()))
    }

    /// A PNG as a Bevy [`Image`] (`Rgba8UnormSrgb`; sampling and atlas
    /// layout are WP-3.2's).
    pub fn bevy_image(&self, name: &str) -> Result<Image, AssetError> {
        let (w, h, data) = self.rgba(name)?;
        Ok(Image::new(
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ))
    }

    /// `StringTranslate`'s table from `/lang/en_US.lang` and
    /// `/lang/stats_US.lang` (a missing file loads nothing, as
    /// `Properties.load` of a null stream throws and is printed).
    // src: StringTranslate.java:10-18
    pub fn string_translate(&self) -> StringTranslate {
        StringTranslate::parse(
            self.get(EN_US_LANG).unwrap_or_default(),
            self.get(STATS_US_LANG).unwrap_or_default(),
        )
    }
}

/// The translation table (`StringTranslate.getInstance()`).
#[derive(Resource, Clone, Debug, Default)]
pub struct Lang(pub StringTranslate);

/// The `resources/` directory: vanilla's `ThreadDownloadResources` target
/// (`<mcDataDir>/resources`), whose `sound/`, `newsound/`, `streaming/`,
/// `music/`, `newmusic/` subdirectories `Minecraft.installResource`
/// (Minecraft.java:1380-1396) hands to the sound manager. Downloading is
/// deferred (D2); the directory is the user's.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct ResourcesDir {
    root: PathBuf,
}

impl ResourcesDir {
    /// Checks that `root` is a directory.
    pub fn open(root: &Path) -> Result<Self, AssetError> {
        if root.is_dir() {
            Ok(ResourcesDir {
                root: root.to_path_buf(),
            })
        } else {
            Err(AssetError::Resources {
                path: root.to_path_buf(),
            })
        }
    }

    /// The directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The file of a resource name as `installResource` receives it
    /// (`"newsound/random/click.ogg"`, `/`-separated, relative), whether or
    /// not it exists.
    pub fn resolve(&self, name: &str) -> PathBuf {
        let mut p = self.root.clone();
        for c in name.trim_start_matches('/').split('/') {
            p.push(c);
        }
        p
    }

    /// `installResource`'s routing (Minecraft.java:1380-1396): the
    /// subdirectory before the first `/` decides the sound pool, and the
    /// rest is the sound's name. `None` for other directories.
    // src: Minecraft.java:1380-1396
    pub fn install_resource_kind(name: &str) -> Option<(SoundPoolKind, &str)> {
        let var3 = name.find('/')?;
        let var4 = &name[..var3];
        let var1 = &name[var3 + 1..];
        let kind = if var4.eq_ignore_ascii_case("sound") || var4.eq_ignore_ascii_case("newsound") {
            SoundPoolKind::Sound
        } else if var4.eq_ignore_ascii_case("streaming") {
            SoundPoolKind::Streaming
        } else if var4.eq_ignore_ascii_case("music") || var4.eq_ignore_ascii_case("newmusic") {
            SoundPoolKind::Music
        } else {
            return None;
        };
        Some((kind, var1))
    }
}

/// The three `SoundManager` pools a resource can go to
/// (`addSound`/`addStreaming`/`addMusic`; WP-3.8 owns them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundPoolKind {
    /// `soundPoolSounds`.
    Sound,
    /// `soundPoolStreaming`.
    Streaming,
    /// `soundPoolMusic`.
    Music,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_jar_names_the_path() {
        let e = JarAssets::open(Path::new("/nonexistent/minecraft.jar")).unwrap_err();
        let s = e.to_string();
        assert!(s.contains("/nonexistent/minecraft.jar"), "{s}");
        assert!(s.contains("--jar"), "{s}");
        let e = ResourcesDir::open(Path::new("/nonexistent/resources")).unwrap_err();
        assert!(e.to_string().contains("/nonexistent/resources"));
    }

    #[test]
    fn install_resource_routing() {
        assert_eq!(
            ResourcesDir::install_resource_kind("newsound/random/click.ogg"),
            Some((SoundPoolKind::Sound, "random/click.ogg"))
        );
        assert_eq!(
            ResourcesDir::install_resource_kind("Music/calm1.ogg"),
            Some((SoundPoolKind::Music, "calm1.ogg"))
        );
        assert_eq!(
            ResourcesDir::install_resource_kind("streaming/13.ogg"),
            Some((SoundPoolKind::Streaming, "13.ogg"))
        );
        assert_eq!(ResourcesDir::install_resource_kind("sound3/x.ogg"), None);
        assert_eq!(ResourcesDir::install_resource_kind("nodir"), None);
    }

    /// Against the real jar when it is present (D8: it is never committed).
    #[test]
    fn the_default_jar_when_present() {
        let path = crate::paths::default_jar();
        if !path.exists() {
            return;
        }
        let jar = JarAssets::open(&path).unwrap();
        let (w, h, px) = jar.rgba(TERRAIN_PNG).unwrap();
        assert_eq!((w, h), (256, 256));
        assert_eq!(px.len(), 256 * 256 * 4);
        let t = jar.string_translate();
        assert_eq!(t.translate_key("gui.done"), "Done");
        assert_eq!(t.translate_key("key.forward"), "Forward");
        assert_eq!(t.translate_key("stat.generalButton"), "General");
        assert!(jar.get(ACHIEVEMENT_MAP_TXT).is_some());
        assert!(jar.get("/net/minecraft/client/Minecraft.class").is_none());
    }
}
