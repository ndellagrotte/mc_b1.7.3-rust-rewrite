//! `World.tick()` and the per-tick block systems (PLAN_v4 §3.3).
//!
//! Order, from World.java:1745-1780: `updateWeather` → all-players-asleep
//! check → `SpawnerAnimals.performSpawning` → `unload100OldestChunks` →
//! sky-light recalculation → `worldTime + 1` with autosave → `TickUpdates`
//! (scheduled block ticks) → `updateBlocksAndPlayCaveSounds` (random ticks).
//!
//! Fully transcribed: `tick`, `updateWeather`, `TickUpdates`,
//! `updateBlocksAndPlayCaveSounds` (except the parts below), the chunk-cache
//! `unload100OldestChunks`, `saveWorld`.
//! Recorded stubs ([`World::skipped_systems`]): `SpawnerAnimals.performSpawning`
//! and `performSleepSpawning` (WP-2.3), `isAllPlayersFullyAsleep` /
//! `wakeUpAllPlayers` (need `EntityPlayer`, WP-2.5), and the biome tests of
//! lightning and snow/ice (WP-1.4; `EntityLightningBolt` is WP-2.4). Each stub sits exactly
//! where its Java code runs and draws no random numbers, so the RNG streams
//! stay aligned with vanilla whenever the stubbed branch would not have run.

use mc_core::java_hash::ChunkCoordIntPair;
use mc_core::math::floor_double;

use crate::block::{self, dispatch as block_dispatch};
use crate::world::World;
use crate::world::chunk::block_index;
use crate::world::events::WorldEvent;
use crate::world::lighting::EnumSkyBlock;

impl World {
    /// `tick()`: one world tick after the entities (the client calls
    /// `updateEntities` first).
    // src: World.java:1745-1780
    pub fn tick(&mut self) {
        self.update_weather();
        if self.is_all_players_fully_asleep() {
            // src: World.java:1748-1759 (sleep check)
            let mut var1 = false;
            if self.spawn_hostile_mobs && self.difficulty_setting >= 1 {
                self.skip("SpawnerAnimals.performSleepSpawning");
                var1 = false;
            }
            if !var1 {
                let var2 = self.world_info.world_time + 24000;
                self.world_info.world_time = var2 - var2 % 24000;
                self.skip("World.wakeUpAllPlayers");
            }
        }

        // src: World.java:1761. Without players there are no eligible chunks
        // and it draws nothing (SpawnerAnimals.java:19-38), so it is only a gap
        // when a player exists.
        if (self.spawn_hostile_mobs || self.spawn_peaceful_mobs) && !self.player_entities.is_empty()
        {
            self.skip("SpawnerAnimals.performSpawning");
        }
        // src: World.java:1762
        self.unload_100_oldest_chunks();
        // src: World.java:1763-1770
        let var4 = self.calculate_skylight_subtracted(1.0);
        if var4 != self.skylight_subtracted {
            self.skylight_subtracted = var4;
            self.events.push(WorldEvent::UpdateAllRenderers);
        }

        // src: World.java:1772-1777
        let var2 = self.world_info.world_time + 1;
        if var2 % self.autosave_period as i64 == 0 {
            self.save_world(false);
        }
        self.world_info.world_time = var2;
        // src: World.java:1778-1779
        self.tick_updates(false);
        self.update_blocks_and_play_cave_sounds();
    }

    /// `isAllPlayersFullyAsleep()`: needs `EntityPlayer.isPlayerFullyAsleep`
    /// (WP-2.5). `allPlayersSleeping` is only set by the player code, so with
    /// no transcribed players it is false and this returns false, as vanilla
    /// does for a world without sleeping players.
    // src: World.java:2409-2426
    fn is_all_players_fully_asleep(&mut self) -> bool {
        if self.all_players_sleeping && !self.multiplayer_world {
            self.skip("World.isAllPlayersFullyAsleep");
        }
        false
    }

    /// `updateWeather()`: rain and thunder timers and strengths. Draws
    /// `rand.nextInt` whenever a timer runs out (on the first tick of a new
    /// world, both do).
    // src: World.java:1792-1859
    #[allow(clippy::manual_clamp)] // Java's two ifs, kept (same result, NaN included)
    pub fn update_weather(&mut self) {
        if self.world_provider.has_no_sky {
            return;
        }
        if self.field_27168_f > 0 {
            self.field_27168_f -= 1;
        }

        let mut var1 = self.world_info.thunder_time;
        if var1 <= 0 {
            if self.world_info.thundering {
                self.world_info.thunder_time = self.rand.next_int_bounded(12000) + 3600;
            } else {
                self.world_info.thunder_time = self.rand.next_int_bounded(168000) + 12000;
            }
        } else {
            var1 -= 1;
            self.world_info.thunder_time = var1;
            if var1 <= 0 {
                self.world_info.thundering = !self.world_info.thundering;
            }
        }

        let mut var2 = self.world_info.rain_time;
        if var2 <= 0 {
            if self.world_info.raining {
                self.world_info.rain_time = self.rand.next_int_bounded(12000) + 12000;
            } else {
                self.world_info.rain_time = self.rand.next_int_bounded(168000) + 12000;
            }
        } else {
            var2 -= 1;
            self.world_info.rain_time = var2;
            if var2 <= 0 {
                self.world_info.raining = !self.world_info.raining;
            }
        }

        self.prev_raining_strength = self.raining_strength;
        if self.world_info.raining {
            self.raining_strength = (self.raining_strength as f64 + 0.01) as f32;
        } else {
            self.raining_strength = (self.raining_strength as f64 - 0.01) as f32;
        }
        if self.raining_strength < 0.0 {
            self.raining_strength = 0.0;
        }
        if self.raining_strength > 1.0 {
            self.raining_strength = 1.0;
        }

        self.prev_thundering_strength = self.thundering_strength;
        if self.world_info.thundering {
            self.thundering_strength = (self.thundering_strength as f64 + 0.01) as f32;
        } else {
            self.thundering_strength = (self.thundering_strength as f64 - 0.01) as f32;
        }
        if self.thundering_strength < 0.0 {
            self.thundering_strength = 0.0;
        }
        if self.thundering_strength > 1.0 {
            self.thundering_strength = 1.0;
        }
    }

    /// `TickUpdates(force)`: runs up to 1000 due scheduled ticks in `TreeSet`
    /// order; returns whether any remain.
    // src: World.java:1967-1995
    pub fn tick_updates(&mut self, force: bool) -> bool {
        let mut var2 = self.scheduled_tick_tree_set.len();
        assert_eq!(
            var2,
            self.scheduled_tick_set.len(),
            "TickNextTick list out of synch"
        );
        if var2 > 1000 {
            var2 = 1000;
        }
        for _ in 0..var2 {
            let var4 = *self
                .scheduled_tick_tree_set
                .first()
                .expect("size checked above");
            if !force && var4.scheduled_time > self.world_info.world_time {
                break;
            }
            self.scheduled_tick_tree_set.remove(&var4);
            self.scheduled_tick_set.remove(&var4.key());
            let var5 = 8;
            if self.check_chunks_exist(
                var4.x_coord - var5,
                var4.y_coord - var5,
                var4.z_coord - var5,
                var4.x_coord + var5,
                var4.y_coord + var5,
                var4.z_coord + var5,
            ) {
                let var6 = self.get_block_id(var4.x_coord, var4.y_coord, var4.z_coord);
                if var6 == var4.block_id && var6 > 0 {
                    block_dispatch::update_tick(
                        self,
                        var6,
                        var4.x_coord,
                        var4.y_coord,
                        var4.z_coord,
                        block_dispatch::RandomSource::WorldRand,
                    );
                }
            }
        }
        !self.scheduled_tick_tree_set.is_empty()
    }

    /// One step of the random-tick LCG: `field_9437_g = field_9437_g * 3 +
    /// 1013904223` (Java `int` overflow).
    // src: World.java:1903, :1921, :1934, :1953
    fn next_lcg(&mut self) -> i32 {
        self.field_9437_g = self.field_9437_g.wrapping_mul(3).wrapping_add(1013904223);
        self.field_9437_g
    }

    /// `updateBlocksAndPlayCaveSounds()`: for every chunk within 9 of a player
    /// (in `HashSet` order): a cave-sound attempt, a lightning roll, a
    /// snow/ice roll, and 80 random block ticks.
    // src: World.java:1868-1965
    pub fn update_blocks_and_play_cave_sounds(&mut self) {
        self.positions_to_update.clear();

        // src: World.java:1871-1886
        let players = self.player_entities.clone();
        for pid in players {
            let Some(p) = self.entity(pid) else { continue };
            let var3 = floor_double(p.base.pos_x / 16.0);
            let var4 = floor_double(p.base.pos_z / 16.0);
            let var5 = 9;
            for var6 in -var5..=var5 {
                for var7 in -var5..=var5 {
                    self.positions_to_update
                        .insert(ChunkCoordIntPair::new(var6 + var3, var7 + var4));
                }
            }
        }

        if self.sound_counter > 0 {
            self.sound_counter -= 1;
        }

        let positions: Vec<ChunkCoordIntPair> = self.positions_to_update.iter().copied().collect();
        for var13 in positions {
            let var3 = var13.chunk_x_pos * 16;
            let var4 = var13.chunk_z_pos * 16;
            let (cx, cz) = (var13.chunk_x_pos, var13.chunk_z_pos);
            self.get_chunk_from_chunk_coords(cx, cz);

            // Cave sound (World.java:1902-1918).
            if self.sound_counter == 0 {
                let var6 = self.next_lcg() >> 2;
                let mut var7 = var6 & 15;
                let mut var8 = var6 >> 8 & 15;
                let var9 = var6 >> 16 & 127;
                let var10 = self.chunk_mut(cx, cz).get_block_id(var7, var9, var8);
                var7 += var3;
                var8 += var4;
                if var10 == 0
                    && self.get_full_block_light_value(var7, var9, var8)
                        <= self.rand.next_int_bounded(8)
                    && self.get_saved_light_value(EnumSkyBlock::Sky, var7, var9, var8) <= 0
                {
                    let var11 = self.get_closest_player(
                        var7 as f64 + 0.5,
                        var9 as f64 + 0.5,
                        var8 as f64 + 0.5,
                        8.0,
                    );
                    let far = var11.and_then(|p| self.entity(p)).is_some_and(|p| {
                        p.base.get_distance_sq(
                            var7 as f64 + 0.5,
                            var9 as f64 + 0.5,
                            var8 as f64 + 0.5,
                        ) > 4.0
                    });
                    if far {
                        let pitch = 0.8 + self.rand.next_float() * 0.2;
                        self.play_sound_effect(
                            var7 as f64 + 0.5,
                            var9 as f64 + 0.5,
                            var8 as f64 + 0.5,
                            "ambient.cave.cave",
                            0.7,
                            pitch,
                        );
                        self.sound_counter = self.rand.next_int_bounded(12000) + 6000;
                    }
                }
            }

            // Lightning (World.java:1920-1930).
            if self.rand.next_int_bounded(100000) == 0
                && self.is_raining()
                && self.get_is_thundering()
            {
                let var6 = self.next_lcg() >> 2;
                let var7 = var3 + (var6 & 15);
                let var8 = var4 + (var6 >> 8 & 15);
                let var9 = self.find_top_solid_block(var7, var8);
                if self.can_block_be_rained_on(var7, var9, var8) {
                    // addWeatherEffect(new EntityLightningBolt(…)); field_27168_F = 2 (WP-2.4).
                    self.skip("EntityLightningBolt (World.updateBlocksAndPlayCaveSounds)");
                    self.field_27168_f = 2;
                }
            }

            // Snow and ice (World.java:1933-1950). The first test is the biome's
            // getEnableSnow(), a recorded stub (false) until WP-1.4; the rest
            // draws no random numbers.
            if self.rand.next_int_bounded(16) == 0 {
                let var6 = self.next_lcg() >> 2;
                let var7 = var6 & 15;
                let var8 = var6 >> 8 & 15;
                let _var9 = self.find_top_solid_block(var7 + var3, var8 + var4);
                self.skip("BiomeGenBase.getEnableSnow (World.updateBlocksAndPlayCaveSounds)");
            }

            // Random ticks (World.java:1952-1962).
            for _ in 0..80 {
                let var7 = self.next_lcg() >> 2;
                let var8 = var7 & 15;
                let var9 = var7 >> 8 & 15;
                let var10 = var7 >> 16 & 127;
                let var15 = self.chunk_mut(cx, cz).blocks[block_index(var8, var10, var9)] as i32;
                if block::tick_on_load(var15) {
                    block_dispatch::update_tick(
                        self,
                        var15,
                        var8 + var3,
                        var10,
                        var9 + var4,
                        block_dispatch::RandomSource::WorldRand,
                    );
                }
            }
        }
    }
}
