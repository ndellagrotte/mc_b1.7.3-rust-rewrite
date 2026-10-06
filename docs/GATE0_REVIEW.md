# Gate 0 review notes (Phase 0, WP-0.1 … WP-0.5)

Written 2026-10-06 by the orchestrating session for the owner's Gate 0 review
(PLAN_v4 §5: "§4.4 items 1–3 pass; owner approves the 0.1 tree and the 0.5
skeleton"). Branches, oldest first: `wp-0.1` → `wp-0.2` → `wp-0.3` → `wp-0.4`
→ `wp-0.5`; each is a linear series of "WP-x.y step N" commits on top of the
previous one. Nothing is pushed.

## 1. §4.4 harness acceptance

| Item | Status | Evidence |
|---|---|---|
| 1. Tier A regenerates byte-identically across three runs | pass | `tools/oracle/check-reproducible.sh` (41 files) |
| 2. Perturbing one seed bit fails the matching fixture | pass | `tools/oracle/check-sensitivity.sh` (39/39); Rust side: `golden_tick::flat_item_drop_with_perturbed_seed_fails` |
| 3. A failing comparison prints the first differing tick and field, decoded | pass | `mc_golden::compare` output, exercised by `crates/mc-golden/tests/runner.rs` |
| 4. `testWorld` loads with equal chunk digests; chest at 9,64,−61 holds 2 planks + 4 sticks | oracle side only | `tests/golden/save/testworld` records the chest (slot 0: id 5 ×2, slot 1: id 280 ×4); the Rust reader is WP-1.9 |

## 2. Decisions the orchestrator took (confirm or overturn)

1. **§3.4 rule 5.** Oracle runs with HotSpot math intrinsics disabled
   (`tools/oracle/env.sh`), so `Math.*` == `StrictMath` (fdlibm 5.3);
   `mc_core::jmath` transcribes fdlibm. The real client differs by 1 ulp on
   1–4 % of `sin`/`cos`/`pow` inputs. Recorded in PLAN_v4 §3.4 and §7.
2. **Java `HashMap` order** targets the JDK 8+ algorithm including red-black
   tree bins (vanilla's `positionsToUpdate` creates them). Implemented
   clean-room from a behavioural spec and the fixtures, not from OpenJDK
   (GPLv2+CPE vs this project's CC0).
3. **Trace comparison** is per parsed JSON line (key order irrelevant), not
   raw bytes; `dbg` fields are ignored.
4. **NBT** compounds are written in sorted key order; parity is on parsed
   trees (PLAN_v4 §4.3 Tier S). Only `nbt/empty-root` is byte-identical.
5. **`Entity.rand` rewrite** (PLAN_v4 §8 item 3): not done; hostile-mob
   spawning fixtures wait on the owner's answer. Harness `difficulty`
   defaults to 0.
6. **WP-0.4 scope addition:** `"world": "empty"` scenarios so WP-0.5 could be
   verified before worldgen exists.

## 3. Skeleton questions from the WP-0.5 author

1. Mutable `Block` singleton state (live bounds, leaves `graphicsLevel`,
   `BlockSand.fallInstantly`) is world state: `World::block_statics`. OK?
2. Leaves' `isOpaqueCube()` depends on the client's fancy-graphics setting. The
   oracle (no renderer) uses `false`. Should the game follow the player's
   setting (changes simulation results) or always use `false`?
3. Block reads take `&mut World` because `getBlockId` loads/generates missing
   chunks in Java; `loaded_chunk()` is the non-loading path. Accept?
4. The generator is an enum (`ChunkGenerator`) behind a `ChunkProvider` trait
   for the non-re-entrant methods; `populate` is a function over the world
   (population re-enters chunk loading; the nether reseeds the same Random).
   Accept this instead of a pure trait?
5. Tile entities live in a world store under stable `TileEntityRef` handles;
   lists and the chunk map hold handles. OK?
6. `entity_state` reports the player as `HarnessPlayer` (oracle name) while the
   Rust kind is `EntityPlayerSP`; the driver maps one to the other.
7. Entity data scheme: flat tuple per concrete class
   (`Pig(LivingData, CreatureData, AnimalData, PigData)`), one function per
   class level, `super.x()` as an explicit call. 28 variants.
8. `entity()`/`entity_index()` are linear scans (O(n²) AABB queries); a map is
   planned before the mob WPs.

## 4. Reviewer findings (PLAN_v4 §6 reviewer role)

- mc-core (`aabb`, `vec3d`, `MathHelper`, `JavaRandom`, key hashCodes): no
  behavioural defects; doc nits applied in `wp-0.3` step 9.
- mc-sim skeleton: `move_entity`, `update_blocks_and_play_cave_sounds` and
  `Chunk.set_block_id*` match the Java line by line. Two latent bugs
  (rider dismount while the mount is out of its slot; `lastSaveTime` written
  without a chunk loader) and three nits were reported with exact fixes and
  applied in `wp-0.5` (see the commit "review fixes").

## 5. Fixture status for Phase 1 planning

`cargo test -p mc-sim --test golden_tick` runs every Tier B/C fixture; passing:
`entity/flat-item-drop`. Ignored, each with the missing Java system in its
reason: lighting (WP-1.3: flat-water-flow, flat-sand-falls, flat-torch-light),
`BlockFlowing` (WP-1.5: flat-flowing-water), `EntityPig` (WP-2.2:
flat-pig-falls), and worldgen (WP-1.4) for all generated-terrain fixtures.

## 6. Known gaps in the oracle

Listed in `tools/oracle/census.md` §3: unseeded constructor draws for mob
`getCanSpawnHere`, projectile heading jitter, painting art, lightning; the
`TileEntityDispenser`/`Explosion` Randoms; left-click block breaking and the
player's portal/`pushOutOfBlocks` paths need `Minecraft`.
