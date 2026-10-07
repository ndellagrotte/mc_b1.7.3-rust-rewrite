//! The sound half of `RenderGlobal.playAuxSFX(player, type, x, y, z, data)`
//! (RenderGlobal.java:1358-1416). The particle halves of 2000 (dispenser
//! smoke) and 2001 (block break) are WP-3.7's; they read the same
//! [`mc_sim::world::events::WorldEvent::PlayAuxSfx`].
//!
//! Types 1000-1004 call `worldObj.playSoundEffect`, which comes back to every
//! `IWorldAccess.playSound` — `RenderGlobal.playSound` with its 16-block
//! range check ([`AuxSound::SoundEffect`]); 1005 calls `worldObj.playRecord`
//! → `RenderGlobal.playRecord` ([`AuxSound::Record`]); 2001 calls
//! `sndManager.playSound` directly, with no range check ([`AuxSound::Direct`]).
//!
//! The random draws: 1003's `Math.random()` is a client site (census class
//! C, tools/oracle/census.md) and takes the client's own stream. 1003's and
//! 1004's `worldObj.rand.nextFloat()` draws are made on the world's `Random`
//! in vanilla, from inside the simulation's call; the simulation does not
//! make them (it only queues the event), so the client draws from a stand-in
//! `Random` of its own (see the WP-3.8 report: an interface change).

use mc_core::JavaRandom;

/// One sound decision of `playAuxSFX` (module docs).
#[derive(Clone, Debug, PartialEq)]
pub enum AuxSound {
    /// `worldObj.playSoundEffect(x, y, z, name, volume, pitch)`.
    SoundEffect {
        /// Sound name.
        name: &'static str,
        /// Position.
        x: f64,
        /// Position.
        y: f64,
        /// Position.
        z: f64,
        /// Volume.
        volume: f32,
        /// Pitch.
        pitch: f32,
    },
    /// `mc.sndManager.playSound(name, x, y, z, volume, pitch)`.
    Direct {
        /// Sound name.
        name: String,
        /// Position.
        x: f32,
        /// Position.
        y: f32,
        /// Position.
        z: f32,
        /// Volume.
        volume: f32,
        /// Pitch.
        pitch: f32,
    },
    /// `worldObj.playRecord(name, x, y, z)` (`None` stops the record).
    Record {
        /// `ItemRecord.recordName`, or `None` when `data` is not a record.
        name: Option<&'static str>,
        /// Block.
        x: i32,
        /// Block.
        y: i32,
        /// Block.
        z: i32,
    },
}

/// `playAuxSFX`'s sound for `kind` at block `(x, y, z)` with `data`; `None`
/// for the types without one (2000, unknown types, 2001 with block id 0 or an
/// undefined block — Java would throw on the latter).
///
/// `world_rand` stands in for `worldObj.rand`, `math_random` for
/// `Math.random()` (module docs).
// src: RenderGlobal.java:1358-1416
pub fn aux_sfx_sound(
    kind: i32,
    x: i32,
    y: i32,
    z: i32,
    data: i32,
    world_rand: &mut JavaRandom,
    math_random: &mut JavaRandom,
) -> Option<AuxSound> {
    let at_corner = |name, volume, pitch| AuxSound::SoundEffect {
        name,
        x: x as f64,
        y: y as f64,
        z: z as f64,
        volume,
        pitch,
    };
    match kind {
        // src: RenderGlobal.java:1362-1364 (dispenser fired)
        1000 => Some(at_corner("random.click", 1.0, 1.0)),
        // src: RenderGlobal.java:1365-1367 (dispenser empty)
        1001 => Some(at_corner("random.click", 1.0, 1.2)),
        // src: RenderGlobal.java:1368-1370 (dispenser shot a projectile)
        1002 => Some(at_corner("random.bow", 1.0, 1.2)),
        // src: RenderGlobal.java:1371-1377 (door / trapdoor toggled): open
        // or close at random, not by the door's state.
        1003 => {
            let name = if math_random.next_double() < 0.5 {
                "random.door_open"
            } else {
                "random.door_close"
            };
            Some(AuxSound::SoundEffect {
                name,
                x: x as f64 + 0.5,
                y: y as f64 + 0.5,
                z: z as f64 + 0.5,
                volume: 1.0,
                pitch: world_rand.next_float() * 0.1 + 0.9,
            })
        }
        // src: RenderGlobal.java:1378-1380 (fire put out): the coordinates
        // are float sums widened to double.
        1004 => {
            let pitch = 2.6 + (world_rand.next_float() - world_rand.next_float()) * 0.8;
            Some(AuxSound::SoundEffect {
                name: "random.fizz",
                x: (x as f32 + 0.5) as f64,
                y: (y as f32 + 0.5) as f64,
                z: (z as f32 + 0.5) as f64,
                volume: 0.5,
                pitch,
            })
        }
        // src: RenderGlobal.java:1381-1387 (jukebox): `data` is the record's
        // item id (`shiftedIndex`), 0 to stop.
        1005 => {
            let name = match mc_sim::item::item(data).map(|d| &d.kind) {
                Some(mc_sim::item::ItemKind::Record { record_name }) => Some(*record_name),
                _ => None,
            };
            Some(AuxSound::Record { name, x, y, z })
        }
        // src: RenderGlobal.java:1408-1412 (block broken): `data` is
        // `blockID + metadata * 256`.
        2001 => {
            let var16 = data & 255;
            if var16 <= 0 {
                return None;
            }
            let var17 = mc_sim::block::block(var16)?;
            let s = &var17.step_sound;
            Some(AuxSound::Direct {
                name: s.step_sound_dir(),
                x: x as f32 + 0.5,
                y: y as f32 + 0.5,
                z: z as f32 + 0.5,
                volume: (s.get_volume() + 1.0) / 2.0,
                pitch: s.get_pitch() * 0.8,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(kind: i32, data: i32) -> Option<AuxSound> {
        aux_sfx_sound(
            kind,
            10,
            64,
            -3,
            data,
            &mut JavaRandom::new(5),
            &mut JavaRandom::new(6),
        )
    }

    fn effect(name: &'static str, volume: f32, pitch: f32) -> Option<AuxSound> {
        Some(AuxSound::SoundEffect {
            name,
            x: 10.0,
            y: 64.0,
            z: -3.0,
            volume,
            pitch,
        })
    }

    #[test]
    fn the_fixed_ones() {
        assert_eq!(run(1000, 0), effect("random.click", 1.0, 1.0));
        assert_eq!(run(1001, 0), effect("random.click", 1.0, 1.2));
        assert_eq!(run(1002, 0), effect("random.bow", 1.0, 1.2));
        assert_eq!(run(2000, 4), None);
        assert_eq!(run(999, 0), None);
    }

    #[test]
    fn door_and_fizz_draws() {
        let mut wr = JavaRandom::new(5);
        let mut mr = JavaRandom::new(6);
        let open = mr.next_double() < 0.5;
        let pitch = wr.next_float() * 0.1 + 0.9;
        let Some(AuxSound::SoundEffect {
            name,
            x,
            y,
            z,
            volume,
            pitch: p,
        }) = run(1003, 0)
        else {
            panic!()
        };
        assert_eq!(
            name,
            if open {
                "random.door_open"
            } else {
                "random.door_close"
            }
        );
        assert_eq!((x, y, z, volume, p), (10.5, 64.5, -2.5, 1.0, pitch));

        let mut wr = JavaRandom::new(5);
        let a = wr.next_float();
        let b = wr.next_float();
        let Some(AuxSound::SoundEffect {
            name,
            volume,
            pitch,
            z,
            ..
        }) = run(1004, 0)
        else {
            panic!()
        };
        assert_eq!(name, "random.fizz");
        assert_eq!(volume, 0.5);
        assert_eq!(pitch, 2.6 + (a - b) * 0.8);
        assert_eq!(z, -2.5);
    }

    #[test]
    fn records() {
        let rec = |name| {
            Some(AuxSound::Record {
                name,
                x: 10,
                y: 64,
                z: -3,
            })
        };
        // record13 = 256 + 2000, recordCat = 256 + 2001 (Item.java:378-379)
        assert_eq!(run(1005, 2256), rec(Some("13")));
        assert_eq!(run(1005, 2257), rec(Some("cat")));
        // BlockJukeBox ejecting passes 0 (BlockJukeBox.java:35)
        assert_eq!(run(1005, 0), rec(None));
        assert_eq!(run(1005, 1), rec(None));
    }

    #[test]
    fn block_break() {
        // stone: soundStoneFootstep ("stone", 1.0, 1.0)
        assert_eq!(
            run(2001, 1),
            Some(AuxSound::Direct {
                name: "step.stone".into(),
                x: 10.5,
                y: 64.5,
                z: -2.5,
                volume: 1.0,
                pitch: 0.8,
            })
        );
        // grass: soundGrassFootstep ("grass", 1.0, 1.0), metadata ignored
        let Some(AuxSound::Direct { name, .. }) = run(2001, 2 + 3 * 256) else {
            panic!()
        };
        assert_eq!(name, "step.grass");
        // glass (20): StepSoundStone → "random.glass"
        let Some(AuxSound::Direct { name, .. }) = run(2001, 20) else {
            panic!()
        };
        assert_eq!(name, "random.glass");
        // sand (12): StepSoundSand → "step.gravel"
        let Some(AuxSound::Direct { name, .. }) = run(2001, 12) else {
            panic!()
        };
        assert_eq!(name, "step.gravel");
        // iron block (42): soundMetalFootstep ("stone", 1.0, 1.5)
        // (Block.java:12): pitch 1.5F * 0.8F
        let Some(AuxSound::Direct {
            name,
            volume,
            pitch,
            ..
        }) = run(2001, 42)
        else {
            panic!()
        };
        assert_eq!(name, "step.stone");
        assert_eq!(volume, 1.0);
        assert_eq!(pitch, 1.5_f32 * 0.8);
        assert_eq!(run(2001, 0), None);
        assert_eq!(run(2001, 256), None);
    }
}
