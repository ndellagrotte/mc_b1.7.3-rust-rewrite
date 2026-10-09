//! `World.tick()` and the per-tick block systems (PLAN_v4 §3.3), plus the
//! client-driven `randomDisplayUpdates`.
//!
//! Order, from World.java:1745-1780: `updateWeather` → all-players-asleep
//! check → `SpawnerAnimals.performSpawning` → `unload100OldestChunks` →
//! sky-light recalculation → `worldTime + 1` with autosave → `TickUpdates`
//! (scheduled block ticks) → `updateBlocksAndPlayCaveSounds` (random ticks).
//!
//! Fully transcribed: `tick`, `updateWeather`, `isAllPlayersFullyAsleep`,
//! `wakeUpAllPlayers` (with `EntityPlayer.wakeUpPlayer`, WP-2.5a),
//! `SpawnerAnimals.performSleepSpawning` and `performSpawning`
//! ([`crate::spawner`]), the chunk-cache `unload100OldestChunks`,
//! `saveWorld`, `TickUpdates`, `updateBlocksAndPlayCaveSounds` (cave sounds,
//! the lightning and snow/ice rolls with their biome tests, the strike itself
//! as an `EntityLightningBolt` weather effect, 80 random ticks per chunk) and
//! `randomDisplayUpdates`.
//!
//! Recorded gaps ([`World::skipped_systems`] / [`World::unimplemented_hits`]),
//! each where its Java code runs: the mob constructors the port does not
//! have yet ([`crate::spawner`] module docs).

use mc_core::java_hash::ChunkCoordIntPair;
use mc_core::math::floor_double;

use crate::block::{self, dispatch as block_dispatch};
use crate::spawner;
use crate::world::World;
use crate::world::chunk::block_index;
use crate::world::chunk_provider::chunk_key;
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
                // src: World.java:1750-1752 (SpawnerAnimals.java:173-251)
                let players = self.player_entities.clone();
                var1 = spawner::perform_sleep_spawning(self, &players);
            }
            if !var1 {
                let var2 = self.world_info.world_time + 24000;
                self.world_info.world_time = var2 - var2 % 24000;
                self.wake_up_all_players();
            }
        }

        // src: World.java:1761
        let (hostile, peaceful) = (self.spawn_hostile_mobs, self.spawn_peaceful_mobs);
        spawner::perform_spawning(self, hostile, peaceful);
        // src: World.java:1762. A MinecraftException (the session lock)
        // leaves World.tick here and in saveWorld below, before
        // setWorldTime and the block ticks (Minecraft.java:540-544 catches
        // it); kept in `session_lock_error` for the client.
        if let Err(e) = self.try_unload_100_oldest_chunks() {
            self.session_lock_error = Some(e);
            return;
        }
        // src: World.java:1763-1770
        let var4 = self.calculate_skylight_subtracted(1.0);
        if var4 != self.skylight_subtracted {
            self.skylight_subtracted = var4;
            self.events.push(WorldEvent::UpdateAllRenderers);
        }

        // src: World.java:1772-1777
        let var2 = self.world_info.world_time + 1;
        if var2 % self.autosave_period as i64 == 0
            && let Err(e) = self.try_save_world(false)
        {
            self.session_lock_error = Some(e);
            return;
        }
        self.world_info.world_time = var2;
        // src: World.java:1778-1779
        self.tick_updates(false);
        self.update_blocks_and_play_cave_sounds();
    }

    /// `isAllPlayersFullyAsleep()`: `allPlayersSleeping` (kept by
    /// `updateAllPlayersSleepingFlag`) and every player fully asleep.
    // src: World.java:2409-2426
    pub fn is_all_players_fully_asleep(&self) -> bool {
        if self.all_players_sleeping && !self.multiplayer_world {
            for &pid in &self.player_entities {
                match self.entity(pid).and_then(|p| p.kind.player()) {
                    Some(p) => {
                        if !p.is_player_fully_asleep() {
                            return false;
                        }
                    }
                    None => {
                        self.note_unimplemented(
                            "World.isAllPlayersFullyAsleep over a taken-out player",
                        );
                        return false;
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// `wakeUpAllPlayers()`: clears the flag, wakes every sleeping player
    /// (`wakeUpPlayer(false, false, true)`: the bed becomes the spawn point)
    /// and stops the rain.
    // src: World.java:2395-2407
    pub fn wake_up_all_players(&mut self) {
        self.all_players_sleeping = false;
        let players = self.player_entities.clone();
        for pid in players {
            let sleeping = self
                .entity(pid)
                .and_then(|p| p.kind.player())
                .is_some_and(|p| p.is_player_sleeping());
            if sleeping {
                // src: World.java:2402 (EntityPlayer.java:617-647)
                self.with_entity_mut(pid, |w, p| {
                    crate::entity::player::wake_up_player(p, w, false, false, true)
                });
            }
        }
        self.stop_precipitation();
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
        // src: World.java:1871-1886. The set is cleared and refilled from
        // the players' chunks each tick. A `JavaHashSet`'s layout (and so its
        // iteration order) after `clear()` and a run of inserts depends only
        // on its capacity and the inserted keys, so when the players are in
        // the same chunks as at the last fill and that fill did not grow the
        // table, refilling would rebuild the set it already holds; the port
        // then keeps it (WP-4.3; nothing else writes the set).
        let centres: Vec<(i32, i32)> = self
            .player_entities
            .iter()
            .filter_map(|&pid| self.entity(pid))
            .map(|p| {
                (
                    floor_double(p.base.pos_x / 16.0),
                    floor_double(p.base.pos_z / 16.0),
                )
            })
            .collect();
        let capacity = self.positions_to_update.capacity();
        let unchanged = self
            .positions_filled_from
            .as_ref()
            .is_some_and(|(c, cap)| *c == centres && *cap == capacity);
        if !unchanged {
            self.positions_to_update.clear();
            for &(var3, var4) in &centres {
                let var5 = 9;
                for var6 in -var5..=var5 {
                    for var7 in -var5..=var5 {
                        self.positions_to_update
                            .insert(ChunkCoordIntPair::new(var6 + var3, var7 + var4));
                    }
                }
            }
            self.positions_filled_from = Some((centres, capacity));
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
                    // src: World.java:1927 (the bolt's rand is unseeded in the
                    // oracle, tools/oracle/census.md §3)
                    let bolt = crate::entity::lightning_bolt::new_entity_lightning_bolt(
                        self,
                        var7 as f64,
                        var9 as f64,
                        var8 as f64,
                    );
                    self.add_weather_effect(bolt);
                    self.field_27168_f = 2; // src: World.java:1928
                }
            }

            // Snow and ice (World.java:1933-1950). `var14` is the chunk object
            // Java holds; it stays loaded for the whole iteration.
            if self.rand.next_int_bounded(16) == 0 {
                let var6 = self.next_lcg() >> 2;
                let var7 = var6 & 15;
                let var8 = var6 >> 8 & 15;
                let var9 = self.find_top_solid_block(var7 + var3, var8 + var4);
                let snowy = self
                    .get_world_chunk_manager()
                    .get_biome_gen_at(var7 + var3, var8 + var4)
                    .get_enable_snow();
                if snowy
                    && (0..128).contains(&var9)
                    && self.chunk_mut(cx, cz).get_saved_light_value(
                        EnumSkyBlock::Block,
                        var7,
                        var9,
                        var8,
                    ) < 10
                {
                    let var10 = self.chunk_mut(cx, cz).get_block_id(var7, var9 - 1, var8);
                    let var15 = self.chunk_mut(cx, cz).get_block_id(var7, var9, var8);
                    // Block.snow = 78, Block.ice = 79, Block.waterStill = 9.
                    // `Block.blocksList[var10].blockMaterial` is read only for
                    // var10 != 0 (short-circuit), so material_of is exact.
                    if self.is_raining()
                        && var15 == 0
                        && block_dispatch::can_place_block_at(
                            self,
                            78,
                            var7 + var3,
                            var9,
                            var8 + var4,
                        )
                        && var10 != 0
                        && var10 != 79
                        && block::material_of(var10).get_is_solid()
                    {
                        self.set_block_with_notify(var7 + var3, var9, var8 + var4, 78);
                    }
                    if var10 == 9
                        && self
                            .chunk_mut(cx, cz)
                            .get_block_metadata(var7, var9 - 1, var8)
                            == 0
                    {
                        self.set_block_with_notify(var7 + var3, var9 - 1, var8 + var4, 79);
                    }
                }
            }

            // Random ticks (World.java:1952-1962). Java reads `var14.blocks`
            // of the chunk it fetched above for all 80 draws; the port looks
            // the chunk up once per run of draws that hit no ticking block
            // and again after each `updateTick`, which may load chunks into
            // the map (nothing unloads during a tick in singleplayer), so the
            // reads and the draw order are the Java's (WP-4.3: one map
            // lookup per tick instead of one per draw).
            let key = chunk_key(cx, cz);
            let mut left = 80;
            while left > 0 {
                let mut hit = None;
                let blocks = &self
                    .chunk_provider
                    .chunk_map
                    .get(&key)
                    .expect("fetched by get_chunk_from_chunk_coords above")
                    .blocks;
                // `next_lcg`'s state in a local while `blocks` is borrowed,
                // stored back before any `updateTick` and after the draws.
                let mut lcg = self.field_9437_g;
                while left > 0 {
                    left -= 1;
                    // src: World.java:1953
                    lcg = lcg.wrapping_mul(3).wrapping_add(1013904223);
                    let var7 = lcg >> 2;
                    let var8 = var7 & 15;
                    let var9 = var7 >> 8 & 15;
                    let var10 = var7 >> 16 & 127;
                    let var15 = blocks[block_index(var8, var10, var9)] as i32;
                    if block::tick_on_load(var15) {
                        hit = Some((var15, var8, var10, var9));
                        break;
                    }
                }
                self.field_9437_g = lcg;
                if let Some((var15, var8, var10, var9)) = hit {
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

    /// `randomDisplayUpdates(x, y, z)`: the client asks 1000 random blocks
    /// within 15 of the player (`x ± (nextInt(16) - nextInt(16))` per axis,
    /// six `World.rand` draws each, 6000 per call) for `randomDisplayTick`
    /// particles. Run by the client after `World.tick` (Minecraft.java:1167-1169).
    /// Loads chunks it touches (`getBlockId`). The blocks get the
    /// [`World::display_random`] stream where Java passes a fresh unseeded
    /// `new Random()` (World.java:1999).
    // src: World.java:1997-2011
    pub fn random_display_updates(&mut self, var1: i32, var2: i32, var3: i32) {
        let var4 = 16;
        let mut var5 = std::mem::replace(
            &mut self.display_random,
            mc_core::java_random::JavaRandom::new(0),
        );
        for _var6 in 0..1000 {
            let var7 = var1 + self.rand.next_int_bounded(var4) - self.rand.next_int_bounded(var4);
            let var8 = var2 + self.rand.next_int_bounded(var4) - self.rand.next_int_bounded(var4);
            let var9 = var3 + self.rand.next_int_bounded(var4) - self.rand.next_int_bounded(var4);
            let var10 = self.get_block_id(var7, var8, var9);
            if var10 > 0 {
                block_dispatch::random_display_tick(self, var10, var7, var8, var9, &mut var5);
            }
        }
        self.display_random = var5;
    }
}
