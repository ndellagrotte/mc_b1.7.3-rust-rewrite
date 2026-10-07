//! `SoundPool` and `SoundPoolEntry` (SoundPool.java, SoundPoolEntry.java):
//! the sounds of one `resources/` category, by name.
//!
//! A file's pool name is its path relative to the category folder, cut at
//! the first `.`, without its trailing digits (when the pool strips them),
//! with `/` turned into `.`: `step/grass1.ogg` → `step.grass`. Every file of
//! a name is a variant; a play picks one at random with the pool's own
//! `Random`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mc_core::JavaRandom;

/// `SoundPoolEntry`: one file of a pool.
// src: SoundPoolEntry.java:5-13
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoundPoolEntry {
    /// `soundName`: the file's name relative to its category folder, as
    /// `installResource` passed it (`"step/grass1.ogg"`). The sound library
    /// identifies (and caches) a sound by this name.
    pub sound_name: String,
    /// `soundUrl`: the file.
    pub path: PathBuf,
}

/// Why `addSound` failed. In Java the `substring`/`charAt` throws, and
/// `ThreadDownloadResources.loadResource` catches it and prints
/// `"Failed to add " + name` (ThreadDownloadResources.java:74-78).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddSoundError(pub String);

/// `SoundPool`.
// src: SoundPool.java:11-16
#[derive(Clone, Debug)]
pub struct SoundPool {
    /// `rand`: `new Random()` (the caller passes its seed).
    rand: JavaRandom,
    /// `nameToSoundPoolEntriesMapping`: name → indices into `all` (Java
    /// keeps the same entry objects in both collections). Lookup only.
    name_to_entries: BTreeMap<String, Vec<usize>>,
    /// `allSoundPoolEntries`, in insertion order.
    all: Vec<SoundPoolEntry>,
    /// `numberOfSoundPoolEntries`.
    pub number_of_sound_pool_entries: i32,
    /// `field_1657_b`: whether `addSound` strips the trailing digits, so
    /// that numbered files become variants of one name. `true` by default;
    /// `SoundManager.loadSoundSettings` clears it for the streaming pool, whose
    /// record names (`"13"`) are digits (SoundManager.java:27).
    // src: SoundPool.java:16
    pub field_1657_b: bool,
}

impl SoundPool {
    /// `new SoundPool()` with its `new Random()`'s seed.
    pub fn new(seed: i64) -> Self {
        SoundPool {
            rand: JavaRandom::new(seed),
            name_to_entries: BTreeMap::new(),
            all: Vec::new(),
            number_of_sound_pool_entries: 0,
            field_1657_b: true,
        }
    }

    /// The pool name of a file name (`addSound`'s first half).
    ///
    /// `Character.isDigit` also accepts non-ASCII decimal digits; only ASCII
    /// digits are stripped here (the resource names are ASCII).
    // src: SoundPool.java:20-28
    pub fn pool_name(name: &str, strip_digits: bool) -> Result<String, AddSoundError> {
        let fail = || AddSoundError(name.to_string());
        // var1.substring(0, var1.indexOf(".")): no '.' → index -1 → throws.
        let dot = name.find('.').ok_or_else(fail)?;
        let mut var1 = &name[..dot];
        if strip_digits {
            // while (Character.isDigit(var1.charAt(var1.length() - 1))):
            // an empty or all-digit name ends at charAt(-1), which throws.
            loop {
                let last = var1.chars().next_back().ok_or_else(fail)?;
                if !last.is_ascii_digit() {
                    break;
                }
                var1 = &var1[..var1.len() - 1];
            }
        }
        // var1.replaceAll("/", ".")
        Ok(var1.replace('/', "."))
    }

    /// `addSound(name, file)`.
    // src: SoundPool.java:18-42
    pub fn add_sound(
        &mut self,
        name: &str,
        path: PathBuf,
    ) -> Result<&SoundPoolEntry, AddSoundError> {
        let var3 = name.to_string();
        let key = Self::pool_name(name, self.field_1657_b)?;
        let idx = self.all.len();
        self.all.push(SoundPoolEntry {
            sound_name: var3,
            path,
        });
        self.name_to_entries.entry(key).or_default().push(idx);
        self.number_of_sound_pool_entries = self.number_of_sound_pool_entries.wrapping_add(1);
        Ok(&self.all[idx])
    }

    /// `getRandomSoundFromSoundPool(name)`: one variant, `rand.nextInt(n)`;
    /// `None` for an unknown name (no draw).
    // src: SoundPool.java:44-47
    pub fn get_random_sound_from_sound_pool(&mut self, name: &str) -> Option<&SoundPoolEntry> {
        let var2 = self.name_to_entries.get(name)?;
        let i = var2[self.rand.next_int_bounded(var2.len() as i32) as usize];
        Some(&self.all[i])
    }

    /// `getRandomSound()`: any entry, `rand.nextInt(size)`; `None` (no draw)
    /// when empty.
    // src: SoundPool.java:49-51
    pub fn get_random_sound(&mut self) -> Option<&SoundPoolEntry> {
        if self.all.is_empty() {
            return None;
        }
        let i = self.rand.next_int_bounded(self.all.len() as i32) as usize;
        Some(&self.all[i])
    }

    /// The variants of a name, in insertion order (for logs and tests).
    pub fn variants(&self, name: &str) -> Vec<&SoundPoolEntry> {
        self.name_to_entries
            .get(name)
            .map(|v| v.iter().map(|&i| &self.all[i]).collect())
            .unwrap_or_default()
    }

    /// The number of distinct names.
    pub fn name_count(&self) -> usize {
        self.name_to_entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_mapping() {
        let n = |s, strip| SoundPool::pool_name(s, strip);
        assert_eq!(n("step/grass1.ogg", true).unwrap(), "step.grass");
        assert_eq!(n("random/click.ogg", true).unwrap(), "random.click");
        assert_eq!(n("random/explode12.ogg", true).unwrap(), "random.explode");
        assert_eq!(
            n("random/wood click.ogg", true).unwrap(),
            "random.wood click"
        );
        assert_eq!(n("calm1.ogg", true).unwrap(), "calm");
        assert_eq!(n("mob/zombie/say3.ogg", true).unwrap(), "mob.zombie.say");
        // the streaming pool keeps the digits
        assert_eq!(n("13.mus", false).unwrap(), "13");
        assert_eq!(
            n("where are we now.mus", false).unwrap(),
            "where are we now"
        );
        // the first '.' cuts, even in a directory name
        assert_eq!(n("a.b/c1.ogg", true).unwrap(), "a");
        // Java throws: no '.', an all-digit name, an empty name
        assert!(n("noext", true).is_err());
        assert!(n("13.mus", true).is_err());
        assert!(n(".ogg", true).is_err());
        assert_eq!(n(".ogg", false).unwrap(), "");
    }

    #[test]
    fn variants_and_counts() {
        let mut p = SoundPool::new(0);
        for f in ["step/grass1.ogg", "step/grass2.ogg", "step/stone1.ogg"] {
            p.add_sound(f, PathBuf::from(f)).unwrap();
        }
        assert!(p.add_sound("bad", PathBuf::new()).is_err());
        assert_eq!(p.number_of_sound_pool_entries, 3);
        assert_eq!(p.name_count(), 2);
        let v: Vec<_> = p
            .variants("step.grass")
            .iter()
            .map(|e| e.sound_name.clone())
            .collect();
        assert_eq!(v, ["step/grass1.ogg", "step/grass2.ogg"]);
        assert!(p.get_random_sound_from_sound_pool("step.dirt").is_none());
    }

    /// The pick is `rand.nextInt(variants)` on the pool's own `Random`, one
    /// draw per play, none for a miss.
    #[test]
    fn variant_selection_follows_java_random() {
        let names = ["x/a1.ogg", "x/a2.ogg", "x/a3.ogg", "x/a4.ogg"];
        let mut p = SoundPool::new(42);
        for f in names {
            p.add_sound(f, PathBuf::from(f)).unwrap();
        }
        let mut r = JavaRandom::new(42);
        for _ in 0..50 {
            assert!(p.get_random_sound_from_sound_pool("x.b").is_none());
            let want = names[r.next_int_bounded(4) as usize];
            assert_eq!(
                p.get_random_sound_from_sound_pool("x.a")
                    .unwrap()
                    .sound_name,
                want
            );
        }
        // getRandomSound draws over all entries
        let want = names[r.next_int_bounded(4) as usize];
        assert_eq!(p.get_random_sound().unwrap().sound_name, want);
        assert!(SoundPool::new(1).get_random_sound().is_none());
    }

    /// `java.util.Random(42).nextInt(4)` sequence, written out (not derived
    /// from `JavaRandom`), so the pick order itself is pinned.
    #[test]
    fn variant_selection_known_sequence() {
        let names = ["x/a1.ogg", "x/a2.ogg", "x/a3.ogg", "x/a4.ogg"];
        let mut p = SoundPool::new(42);
        for f in names {
            p.add_sound(f, PathBuf::from(f)).unwrap();
        }
        let got: Vec<_> = (0..8)
            .map(|_| {
                p.get_random_sound_from_sound_pool("x.a")
                    .unwrap()
                    .sound_name
                    .clone()
            })
            .collect();
        // new Random(42): nextInt(4) = 2, 0, 2, 0, 1, 3, 1, 2 (printed by a
        // JVM).
        let idx = [2, 0, 2, 0, 1, 3, 1, 2];
        let want: Vec<_> = idx.iter().map(|&i| names[i].to_string()).collect();
        assert_eq!(got, want);
    }
}
