# Performance pass (WP-4.3)

Written 2026-10-09 by the WP-4.3 transcriber for the owner's review. PLAN_v4 §5
Phase 4 row 4.3 asks for a performance pass (chunk meshing, lighting,
incremental region writes, the simulation's tick rate) and packaging through the
template's release workflow, with measured before/after numbers.

Branch `wp-4.3`, forked from `phase-4` at `70cdb29` (WP-4.2 step 5). Commits:

| Step | Commit | What |
|---|---|---|
| 1 | `75aac07` | Simulation hot spots in `mc-sim`, fixture-neutral |
| 2 | `0c321b9` | Client measurement (`perf`), `--vsync off`, write-behind saves with incremental region writes |
| 3 | `46f6fdc` | `flate2` on the `zlib-rs` backend |
| 4 | `66a855a` | Packaging: default paths, `docs/RUNNING.md`, release workflow |
| 5 | (this file) | This report |

(The table's step 5 is the commit that adds this file.)

**Results in one paragraph.** The simulation is now **3.0–5.4× faster** than
before this WP at the test profile's opt-level 1 and **2.6–3.6× faster** in the
`dist` build. It runs the soak fixtures at 2,000–3,000 ticks per second, about
the oracle's ~2,300–2,400. In the client, autosaves no longer stall the game
thread. Before, every autosave (24 chunks every 40 ticks) rewrote and synced
whole region files on the game thread: about 26 MB and 0.1–0.2 s per autosave
on `testWorld`, with frames of 0.13–1.3 s. Now the game thread writes nothing
and the disk sees ~0.27 MB per autosave. The longest frame on the measured
scenes fell from 82–208 ms (up to 1,264 ms on a busier run) to 10–28 ms. A new
seed-123 world reaches the game in 1.2–1.4 s instead of 3.4–4.1 s. Every
default fixture and every soak fixture still passes, the oracle regenerates
without drift, and static scenes are pixel-identical to `70cdb29`.

## 1. Gate commands

Run in the worktree at the step 4 commit (Arch Linux, kernel 7.2.8, AMD Ryzen 7
7700X 8 cores / 16 threads, 32 MB L3, NVIDIA RTX 3080 driver 615.71, Wayland).

| Check | Result |
|---|---|
| `cargo build -p mc-app` | pass |
| `cargo build --profile dist -p mc-app` | pass (9 min 52 s from a cold `dist` directory) |
| `cargo clippy --workspace --all-targets --all-features -- -Dwarnings` | clean |
| `cargo fmt --all --check` | clean |
| `cargo test --workspace` | pass: 1,123 passed, 0 failed, 5 ignored (the soaks) (§1.1) |
| `cargo check -p mc-core -p mc-nbt -p mc-sim --target wasm32-unknown-unknown` | pass (with `zlib-rs`) |
| `cargo test -p mc-golden` (221 default fixtures) | pass |
| `cargo test -p mc-golden --test golden_soak -- --ignored` (5 soaks) | pass, test and `dist` profiles |
| `sh tools/oracle/regen.sh` | no drift (`git status --short tests/golden` empty) |
| Static scenes, `70cdb29` vs final (§4.4) | 0 differing pixels in 3 of 3 |

### 1.1 Tests added

`mc-client`: 440 unit tests (431 at `70cdb29`). New: `perf::tests` (3: nearest-rank p95, the bounded window, the phase
timer and F3 lines), `storage::tests` (5 new: changed-sector ranges, journal round
trip, write-behind and incremental writes, journal replay after a simulated crash,
`level.dat`'s write/rename dance on the queue against the synchronous storage; the
save-handler test now runs on both storages), `paths::tests` (1 new: data
directory first, development fallbacks only when they exist).

## 2. Method

### 2.1 Simulation

- **Profiler.** `perf` is not installed and `perf_event_paranoid` is 2, so
  `samply` (installed) cannot record. A small crate under
  `build/screens/wp43/scripts/simprof/` runs one golden fixture through
  `mc_golden::tick_driver::drive_in` under `pprof`'s SIGPROF sampler (997 Hz;
  user-space only, no `perf_event` access needed). It prints self and inclusive
  samples by function and self samples by source line. It is built with
  opt-level 3 and line tables in its own target directory and lives outside the
  workspace. Nothing in `mc-sim` or `mc-golden` was instrumented.
- **Timing.** The `golden_soak` test binaries of `70cdb29` were saved before any
  change. Then those binaries and the final ones ran one after another on an
  otherwise idle machine, `--ignored --test-threads=1`, once in the test profile
  (the workspace's `dev`: opt-level 1, dependencies at 3) and once in `dist`
  (opt-level 3, fat LTO, one codegen unit). Each test prints its ticks per
  second (`golden_soak.rs`). Wall time includes building the world and
  comparing the trace.

### 2.2 Client

- **Counters** (`crates/mc-client/src/perf.rs`, a project addition):
  - The **frame time** is the time between two app updates.
  - The **update time** is the main schedule's time from `First` to `Last`.
    It is the frame's CPU work on the main thread. Bevy renders the previous
    frame on its own thread, and the wait for that thread and for the display
    is not in this figure.
  - Per-**phase** times come from guards in the client's systems: `sim` =
    `game_frame` (input, game ticks, the frame tail), `terrain`, `entities`,
    `particles`, `gui`, `gui_flush`, `sky`, `hand`.
  - All three are kept over the last 600 frames (min / mean / nearest-rank p95 /
    max), with the start-up milestones and the native storage's counters.
  - They are reported in `dump_debug` (`perf`, `storage`) and in four `[rust]`
    lines in F3's right column, from y = 52 down, below the rows of the left
    column's lines (frame and update min/avg/p95/max, then the phase means
    in two short lines).
- **Uncapped present mode:** `mc_b173 --vsync off` (`PresentMode::AutoNoVsync`).
  The compositor (KWin/Wayland) still paces the window while it is covered or
  hidden. Some runs were capped at 60 fps for that reason, both before and
  after the change. So the comparison uses the **update time**, which pacing
  does not affect. The frame time is reported too, from runs where the window
  was visible.
- **Scenarios** (`build/screens/wp43/scripts/`, run by `run.sh BIN LABEL SCENARIO`
  on a fresh data directory with `--vsync off`, default options: render distance
  Far, fancy graphics, smooth lighting, Balanced):
  - `tw` imports `testWorld`, waits 200 ticks, turns on F3, then:
    - 700 frames standing, then a dump;
    - 400 more ticks standing (10 autosaves), then a dump;
    - looks south (yaw 0, pitch 10) and walks forward for 200 ticks, with
      dumps at 100 and 200 ticks;
    - the dev `save` action, a dump, then quit.
  - `s123` creates world `perf123` with seed 123 and waits 300 ticks (the load
    dump). It then looks at yaw 45, pitch 5 and runs the same steps as `tw`.
  - `title` dumps the start-up milestones from the title screen.
- **Binaries.**
  - *before* is `70cdb29` plus only the measurement code: the step 2
    instrumentation with the step 1 `mc-sim` files reverted and the
    write-behind switched off. The instrumentation alone changes no drawing
    and no timing path.
  - *after* is the step 4 tree.
  - Both are `dev` builds, which is what Gate 3's baseline used. *after* was
    also measured as a `dist` build.
  - Every row below is two runs (`m1`, `m2`).
- **Client profiler:** `build/screens/wp43/scripts/appprof/` is a copy of
  `mc-app`'s `main` linked with `pprof`. It writes folded stacks per thread;
  `fold.py` summarises them.
- **Meshing profiler:** `build/screens/wp43/scripts/meshprof/` generates seed
  123, then snapshots and meshes the 13 × 13 × 8 sections around spawn as
  `terrain_frame` does, timed and sampled. It has an opt-level 1 and an
  opt-level 3 manifest.

## 3. Simulation

### 3.1 What the profile showed (`70cdb29`, `soak/seed-123`, opt-level 3)

| Self samples | Where |
|---|---|
| 65.6 % | `World::update_blocks_and_play_cave_sounds`, nearly all at one line: the random-tick block read `self.chunk_mut(cx, cz).blocks[...]` (`tick.rs:414`), 361 chunks × 80 draws per tick |
| 9.1 % + 2.3 % | `<i32 as Ord>::cmp` and `find_key_index`: the `BTreeMap<i32, Chunk>` behind every chunk lookup (`chunkMap`) |
| 6.7 % | `miniz_oxide` deflate: the autosave's chunk compression |
| 3.7 % / 2.8 % | `NibbleArray::get_nibble`, `Chunk::get_block_id` |
| ~2 % | `JavaHashSet<ChunkCoordIntPair>` inserts: `positionsToUpdate` rebuilt every tick |

The random-tick loop is memory-bound. A standalone benchmark of only those
reads, over 361 boxed 32 KB arrays, costs 12 ns per draw. With
`_mm_prefetch` hints issued for the 80 draws first, it costs 3.7–7.6 ns,
depending on how the arrays lie in memory (§6, question 1).

### 3.2 Changes (step 1, `mc-sim`; step 3, workspace `flate2` features)

1. **`ChunkProvider.chunkMap`** is now a `HashMap<i32, Box<Chunk>>` with a
   fixed multiplicative hasher (`ChunkKeyHasher`). It was a `BTreeMap<i32, Chunk>`.
   Java's map is a `HashMap` used for lookup only (ChunkProvider.java:28-29,
   :45, :73). The port never iterates it either: the save and unload loops
   walk `chunk_list` and `dropped_chunks_set`. So its order is never
   observable (§3.4 rule 4), and the hasher is unseeded (deterministic,
   wasm-safe). `get_chunk_from_chunk_coords` computes the key once.
2. **Random ticks** (World.java:1952-1962):
   - Java holds the chunk (`var14`) for all 80 draws. The port now looks it
     up once per run of draws that hit no ticking block, and again after
     each `updateTick`, which can load chunks.
   - Before, it looked the chunk up on every draw.
   - The LCG state is kept in a local and stored back before each
     `updateTick`.
   - The draws, reads and `updateTick` calls are the same, in the same order.
3. **`positionsToUpdate`** (World.java:1871-1886): the set was cleared and
   refilled with 361 entries every tick. Now it is refilled only when the
   players' chunk coordinates, or the set's capacity at the last fill, have
   changed.
   - A `JavaHashSet` after `clear()` and the same run of inserts at the same
     capacity has the same layout, and so the same iteration order (the
     Java's `HashMap.clear` keeps the table).
   - Nothing else writes the set.
   - The cost was larger than its sample share suggested: the soaks gained
     about 1.5× from this change alone.
4. **Deflate backend:** `flate2` now uses `zlib-rs` (pure Rust, builds for
   wasm) instead of `miniz_oxide`.
   - On the 784 chunks of `testWorld` a chunk deflates in 0.32–0.48 ms
     instead of 0.48–0.65 ms.
   - The compressed bytes differ from `miniz_oxide`'s and from Java's
     zlib's; `miniz_oxide`'s already differed from Java's (checked against
     a `java.util.zip.DeflaterOutputStream`).
   - Tier S compares parsed NBT. Sector offsets and `SizeOnDisk` are masked
     (tests/golden/README.md, `level_dat` and `region_header`). `Cargo.lock`
     is unchanged.

Tried and dropped:

- **Gathering the 80 block ids first, so the reads are independent:** slower,
  because each ticking block forces a re-read of the rest.
- **Loading the 80 cells first as a safe "prefetch":** slower in the
  benchmark (20 ns per draw).

### 3.3 Before / after: soak fixtures, ticks per second (one at a time)

| Fixture (ticks) | test profile before | after | × | `dist` before | after | × |
|---|---|---|---|---|---|---|
| `nether-8675309` (36,000) | 409 | 2,214 | 5.4 | 951 | 3,050 | 3.2 |
| `seed-123` (72,000) | 532 | 2,500 | 4.7 | 991 | 3,007 | 3.0 |
| `seed-8675309` (48,000) | 635 | 2,728 | 4.3 | 921 | 2,621 | 2.8 |
| `seed--7612233452384654969` (48,000) | 593 | 2,033 | 3.4 | 728 | 2,008 | 2.8 |
| `testworld` (48,000) | 630 | 2,361 | 3.7 | 793 | 2,320 | 2.9 |
| all five, wall time | 456 s | 107 s | 4.3 | 289 s | 99 s | 2.9 |

For reference, the oracle runs these at about 2,300–2,400 ticks per second,
JVM start-up included (tests/golden/README.md "Soak"; `seed-8675309`: 19–20 s).
The full parallel `cargo test -p mc-golden --test golden_soak -- --ignored` now
takes 37–47 s; WP-4.2 measured about 2.5 min.

`cargo test --release` uses the `release` profile, which the template sets to
`opt-level = "s"` for the web build. It ran the soaks *slower* than the test
profile in a contended run (231 ticks/s on `nether-8675309`), so it is not the
profile to measure speed with. Use `dist`.

### 3.4 What is left (after, `soak/seed-123`, opt-level 3, 32 s under the sampler)

| Inclusive | Where |
|---|---|
| ~55 % self | the random-tick block reads (memory latency; §6 question 1) |
| 11 % | autosave: chunk NBT and deflate (`zlib-rs`) |
| 7–9 % | chunk generation and population (noise, caves, ores) |
| 4.8 % | `updateEntities` |
| 3.1 % | lighting (`updatingLighting` → `MetadataChunkBlock.updateLight`) |
| 3.6 % | `randomDisplayUpdates` (the harness calls it as the client does) |

Lighting (the relight queue) is 3 % and was left alone.

## 4. Client

### 4.1 Frame and update times (ms: min / mean / p95 / max over 600 frames)

`testWorld`, standing, after 400 ticks (10 autosaves in the window):

| Build | Frame time | Update time | `sim` mean / max |
|---|---|---|---|
| before, dev | 3.1–3.3 mean, p95 5.4–5.5, **max 129–197** | 1.97–2.16 / 5.07–5.17 / **129–196** | 0.46–0.58 / 127–194 |
| after, dev | 2.7–2.8 mean, p95 3.1–3.2, max 13–14 | 1.56–1.59 / 2.40–2.48 / **4–5** | 0.07–0.08 / 2 |
| after, `dist` | 2.2–2.3 mean, p95 2.5–2.7, max 11 | 1.18–1.24 / 1.54–1.99 / 10 | 0.06–0.07 / 9 |

`testWorld`, walking south (200 ticks; new sections load):

| Build | Frame time | Update time |
|---|---|---|
| before, dev | 4.1–4.5 / p95 8.4–9.6 / max 134–202 | 2.90–3.20 / 7.99–9.14 / 133–202 |
| after, dev | 3.3–3.5 / p95 4.5–4.9 / max 13–14 | 2.02–2.12 / 3.85–4.20 / 12 |
| after, `dist` | 2.7 / p95 3.3–3.4 / max 11–12 | 1.53–1.55 / 2.75–2.85 / 11–12 |

Seed 123, new world, Far:

| Moment | before, dev (update) | after, dev (update) | after, `dist` (update) |
|---|---|---|---|
| 300 ticks after arrival (sections still building) | 6.12–6.19 / 10.19–10.63 / 95–169; `sim` mean 2.9–3.0 | 2.13–2.92 / 9.26–9.56 / 15–16; `sim` 0.2–0.4 | 1.78–2.12 / 6.05–7.33 / 24–28 |
| standing, 400 ticks later | 2.38–2.50 / 7.88–8.49 / 82–161 | 1.99–2.04 / 4.72–4.92 / 11–12 | 1.39–1.44 / 2.86–3.18 / 10–11 |
| walking 200 ticks | 2.54–3.48 / 9.00–10.45 / 91–162 | 1.86–2.18 / 4.17–4.76 / 11–12 | 1.39–1.52 / 2.86–3.17 / 10–12 |

The remaining 9–14 ms maxima are game ticks that generate and populate new
chunks. The p95 while sections still build (5–10 ms) is mostly `terrain`: by
design, `updateRenderers` spends up to 1/120 s per frame from
`renderEndNanoTime` on rebuilds (EntityRenderer.java:364-378, "Balanced").

An earlier pair of runs, during a busier period on the desktop, showed the
same effect, larger: before, update-time maxima of 383–1,264 ms on `testWorld`
and a 2,486 ms `sim` frame on the new seed-123 world; after, 17–62 ms
(`build/screens/wp43/{base,s2,s3}_dev`).

### 4.2 Phases (after, dev, `testWorld` standing with F3 on; ms per frame, mean)

| sim | terrain | entities | particles | gui | gui_flush | sky | hand |
|---|---|---|---|---|---|---|---|
| 0.07–0.08 | 0.23 | 0.05 | 0.001 | 0.18 | 0.05 | 0.04 | ~0.01 |

`gui` is 0.02–0.03 ms without F3: the F3 text is most of it. The update time
(~1.6 ms) minus the phases (~0.65 ms) is Bevy's own systems: transform and
visibility propagation, `calculate_bounds`, asset events, and extraction for
the render thread, with up to ~3,000 section entities at Far. `dist` cuts the
total to ~1.2 ms.

### 4.3 Saves

Every `RegionFile.write` hands the whole file to `Storage::write`
(`region.rs` "Storage model"). `FsStorage` wrote that atomically: a synced
temporary file, renamed into place, on the game thread.

What changed (`crates/mc-client/src/storage.rs` docs):

- **A write-behind queue.** The game's `FsStorage` (`with_write_behind`) puts
  `write`, `rename` and `delete` on a queue and returns. A background thread
  (`mc-storage-writer`) executes them in call order.
- **Reads see the queue.** `read` returns the state the queued operations
  lead to. `list` waits for the queue to empty first.
- **Coalescing.** A write of a file that is already queued replaces the queued
  bytes, when nothing in between touches the file. An autosave's 24 chunk
  writes, all to one or two region files, become one or two file writes.
- **Incremental, crash-safe region writes.** A region file is diffed against
  the file on disk by 4096-byte sector. When less than half the file changed,
  the writer:
  1. writes only the changed sectors to a journal next to the file
     (`.mc_journal-r.X.Z.mcr`), atomically;
  2. writes the sectors into the file in place and syncs it;
  3. deletes the journal.

  A crash before the journal's rename leaves the old file. A crash after it
  is finished by the next read, which replays the journal. Replay is
  idempotent. So every file stays old or new, the promise the atomic write
  made. Otherwise the whole file is written atomically, as before.
- `mc-sim` and its `Storage` trait are unchanged, so the Tier S fixtures cannot
  be affected. What reaches the disk is the same bytes; only when and how
  often changes.
- The game flushes the queue before it exits. The import and the tests use
  the synchronous storage.

Per 400 ticks standing, which is 10 autosaves (dev; the `dist` numbers match):

| | Writes on the game thread | Bytes to disk | Time on the game thread | Writer thread |
|---|---|---|---|---|
| `testWorld` before | 250 files | 255–260 MB | 1.1–1.8 s | — |
| `testWorld` after | 0 | 2.7–2.8 MB (journals + sectors) | 0 | 0.22–0.41 s |
| seed 123 before | 250 files | 55–57 MB | 0.7–1.3 s | — |
| seed 123 after | 0 | 2.2–2.6 MB | 0 | 0.19–0.34 s |

The explicit save at the end of `s123` (the dev `save` action, `saveWorld(true)`)
wrote 779–805 files and 440–470 MB on the game thread, in 2.7–5.4 s, before.
After, it writes nothing on the game thread. Over a whole `s123` run, 1,644–1,705
region writes were queued and 1,480–1,586 of them coalesced; 93–135 files were
written through a journal and 11–21 whole.

### 4.4 Pixel-identical check

`build/screens/wp43/scripts/pix.sh BIN LABEL` takes three static screenshots,
with vsync on, at 854 × 480:

- `testWorld`, F1, `set_time 6000`, looking straight down at the chest and the
  workbench after 100 ticks;
- the Options screen;
- the Video Settings screen.

`pngdiff.py` compares decoded pixels. The same binary run twice gives identical
images. Between `70cdb29` and the final tree, **0 of 409,920 pixels differ in
each of the three**. The four `[rust]` F3 lines are the only visual addition,
and they show only with F3 on.

### 4.5 Meshing: measured, left as it was

The 3.3a review asked about four costs. The `meshprof` harness measured them.
Per section, single thread: snapshot 0.055 ms and `updateRenderer` 0.37 ms at
opt-level 1; 0.036 ms and 0.34 ms at opt-level 3.

| Item | Measured | Decision |
|---|---|---|
| `SnapshotTemplate`/`ChunkSnapshot` clones per job (`BlockStatics` 12 KB twice, `WorldChunkManager`) | Sharing both through `Arc`, with the chunk manager copied lazily: snapshot 0.055 → 0.053 ms, `updateRenderer` unchanged | Reverted: within noise, and it added complexity |
| `BTreeMap` lookups per face (`textures`, `bounds`) | 0.2 % of samples | Left |
| `index()` with six comparisons | Rewritten with unsigned compares: no measurable change | Reverted |
| In-flight job cap (16 per task-pool thread) | The worker threads keep up. In game, `snapshot` per rebuild is 0.16 ms (dev) / 0.085 ms (`dist`) on the main thread; meshing is 0.6–0.9 ms per rebuild on the workers, in parallel | Left |
| Mesh asset churn (`meshes.add` per rebuild) | Not visible in the profile | Left |

In game, most of a snapshot's main-thread time is the `ChunkCache`
constructor loading and generating chunks (ChunkCache.java:17-21). That is
simulation work vanilla does at the same point. The meshing workers spend
their time in the AO path's light and opacity reads
(`render_standard_block_with_ambient_occlusion`, 80 %). The next lever would be
per-snapshot caches of light values and opacity flags. It was not taken, because
off-thread meshing keeps up at Far.

The entity renderers (0.05–0.2 ms), the GUI batcher's flush (0.05 ms) and the
particles (≤ 0.01 ms) are too small to show. The clones the 3.5b review noted
(tile entities, the spawner's entity, the falling-sand snapshot) were left as
they were.

### 4.6 Start-up (ms from the process start)

| | Plugin built (jar indexed, options, stats) | Title screen | In game: `testWorld` import | In game: new seed-123 world (generation + 289-chunk preload) |
|---|---|---|---|---|
| before, dev | 207–294 | 245–279 | 419–435 | 3,447–4,117 |
| after, dev | 203–299 | 235–243 | 365–407 | 1,239–1,375 |
| after, `dist` | 188–272 | 226–287 | 360–361 | 1,143–1,176 |

Reading and indexing the jar takes most of the plugin-built time. The new
world's time dropped because the preload (`func_6255_d`, 289 chunks) no longer
waits for synced region writes. The terrain preload's per-frame budget
(`step_pending(25)`) was not changed.

## 5. Packaging (step 4)

- **Default paths.** `--jar` and `--resources` default to
  `<data-dir>/minecraft.jar` and `<data-dir>/resources`. They are resolved
  against `--data-dir` when it is given (`ClientPaths::for_data_dir`).
  - The development fallbacks apply only when that file or directory is
    missing and the fallback itself exists: the workspace's `.reference`
    jar (compiled in from `CARGO_MANIFEST_DIR`) and the Prism instance's
    `resources/`.
  - The D8 errors name the path tried and point to `docs/RUNNING.md`. This
    answers Gate 3 §8 question 5.
- **`docs/RUNNING.md`** covers getting the jar (the Prism library path was
  checked on this machine) and `resources/`, the data directory per platform,
  every flag, the development fallbacks, the `dist` build and the save journal.
  `README.md` and `crates/mc-client/README.md` got additive sections.
- **`release.yaml`:**
  - Every platform builds `cargo build --profile dist -p mc-app`. Before, the
    command had no `-p`, so it built the whole workspace.
  - Each archive also gets `NOTICE` (the Apache-2.0 credit D4 requires),
    `LICENSE` and `RUNNING.md`. The Linux job also installs
    `libxkbcommon-dev`, as `ci.yml` does.
  - The binary name `mc_b173`, the macOS `Info.plist` (`CFBundleExecutable
    mc_b173`), the Windows installer (`target\dist\mc_b173.exe`) and the
    icons (`build/macos/AppIcon.iconset`, `build/windows/icon.rc`) already
    matched. No mobile workflow is left.
  - The `build-web` job is set to `if: false` until Phase 5: `cargo check -p
    mc-app --target wasm32-unknown-unknown` fails (`kira::sound::streaming`).
    `deploy-page.yaml` (manual only) has a note saying the same.
- `ci.yml` already matched the layout and is unchanged.
- **Proof:**
  - `cargo build --profile dist -p mc-app` succeeds (7 min 10 s from the dev
    build's cache). The binary is 75 MB, stripped.
  - The proof below was re-run on the dist build of `effbb31` (the review
    fixes; the first run, at 15:52, used a binary built before the step-4
    commit, so it did not prove the committed tree).
  - The Linux bundle was packed as the workflow packs it: `mc_b173`,
    `assets/`, `credits/`, `NOTICE`, `LICENSE`, `RUNNING.md`.
  - It was unpacked into an empty scratch directory and run there with
    explicit `--jar`/`--resources`. It created world `dist` (seed 123),
    was in game after 1,053 ms and played 200 ticks
    (`build/screens/wp43/dist_fix/01_ingame.png`, `02_f3.png`: the seed-123
    beach and lake, trees, clouds, the sun, the hand and the HUD; with F3,
    the four `[rust]` lines under the left column's rows). It quit and
    saved (`level.dat`, `level.dat_old`, `session.lock`, four region files,
    no journal or temporary left). A second start listed the world
    (`03_select.png`: "dist (10/9/26 7:33 PM, 3.23 MB)").
  - `--jar <scratch>/nope.jar` stops with the path in the message.

## 6. Owner questions

1. **`unsafe` prefetch in the random-tick loop?** About 55 % of a soak tick is
   now the 28,880 random block reads per tick, which are memory-latency bound.
   Issuing `_mm_prefetch` for a chunk's 80 draws before using them (the LCG
   sequence is known in advance) cut the benchmark from 12 ns to 3.7–7.6 ns per
   draw. It needs `unsafe` (`core::arch`; `core::hint::prefetch_read` is still
   unstable), and `mc-sim` has `#![forbid(unsafe_code)]`. That is policy, so it
   was not done. If allowed, it would be one `#[allow(unsafe_code)]` site under
   `cfg(target_arch = "x86_64")` (`mc-sim` would change `forbid` to `deny`). The
   expected gain is ~1.3–1.6× on the soaks.
2. **The `dev` profile's opt-level 1 for workspace crates.** The game and the
   tests run 1.0–1.6× slower than `dist` (§3.3; client update time 1.6 vs 1.2 ms).
   Raising it costs compile time. It is left at 1.
3. **Accept the `zlib-rs` backend** (§3.2 item 4): a dependency feature, not
   behaviour, but a workspace-wide change.
4. **Accept the write-behind storage** (§4.3): a crash or power loss can lose
   the operations still queued (milliseconds' worth). Each file stays old or
   new. Vanilla's in-place `RandomAccessFile` writes can tear a chunk instead.

## 7. Known issues found, not fixed (not caused by this WP)

- **Segfault at exit when quitting within about a second of loading a world**,
  in both the `70cdb29` and the final binaries (4 of 6 tries).
  - The backtrace shows a `bevy_render` compute-pipeline drop on a task-pool
    thread inside `libnvidia-glcore` while the main thread unmaps the driver
    (`coredumpctl`). That is a pipeline compilation still running at
    process exit.
  - The save is written before it happens.
  - It was not seen with later quits.
  - **Fixed by WP-4.9** (`30f897b`, `RenderPlugin::synchronous_pipeline_compilation`
    in `mc-app`): reproduced 8 of 8 times with asynchronous compilation
    (`testWorld` imported, `quit` as soon as the world is in, nested Xwayland;
    the core's crashing thread is in `wgpu_hal::vulkan::Device::create_render_pipeline`
    inside `libnvidia-glcore`, called from `PipelineCache::start_create_render_pipeline`'s
    task), 0 of 20 with synchronous compilation (and 0 of 20 more on Wayland
    with the `g01` world). With nothing compiled off the frame, no pipeline task
    is running when the process exits. The cost is a one-off hitch in the frame
    that first needs a pipeline (the first world frame, the first mob of a kind).
- **Scripts that click "Create New World" twice, 10 frames apart, stay on the
  Select World screen.** A screen opened by a click opens at the next tick hook
  (README "Known deviations"). With a fast `dist` frame rate, 10 frames are
  less than one tick. This affects only dev scripts.
- The compositor paces a covered window even with `--vsync off` (§2.2).
  Since WP-4.9 `off` is the default (vanilla's uncapped Balanced, PARITY_CHECKLIST
  P6); a nested rootful Xwayland presents the Vulkan surface at 60 Hz whatever
  the present mode (the parity harness's display), while the same build runs at
  160-380 fps straight on Wayland.
- **Anaglyph 3D draws one eye** (accepted, WP-4.9). Vanilla runs `renderWorld`'s
  body twice with `glColorMask` per eye and the projection shifted by
  `anaglyphField` (EntityRenderer.java:454-588). Here the anaglyph option
  converts the textures (`refreshTextures`) and draws the first eye's colours
  (`camera::setup_camera_transform(Some(0))`). The second eye would need a second
  world camera with the other shift and a colour-write mask on every world
  material (terrain, entities, particles, sky, clouds, weather, selection, hand),
  each one a pipeline variant, for an option that is off by default; left as a
  documented divergence.
- **`f32` jitter far from the origin; floating-origin design** (WP-4.9, not built).
  Bevy's `Transform`/`GlobalTransform` and `clip_from_world` are `f32`, and the
  client places everything in world space: the camera at the eye
  (`camera::camera_transform`), sections at their corner
  (`terrain::section_transform`), the crack at its block, entity draws at
  `EntityFrame::anchor` (`render_pos`), particles at their anchor, the
  selection box, rain and snow columns, clouds. At |x| ≈ 1,000,000 the `f32`
  step is 1/16 block (at the 30,000,000 border, 2 blocks), so vertices wobble
  as the camera moves. Vanilla avoids it: a section's display list is compiled
  at `posXClip = posX & 1023` and `RenderList` translates by
  `(float)(posXMinus - viewX)` (RenderList.java:17-56, WorldRenderer.java:53-75),
  and entities are drawn at `(float)(entityPos - renderPos)`.
  The design:
  1. A `RenderOrigin` resource: an `IVec3` of multiples of 1024 (vanilla's
     `posXMinus` grid, y = 0), the view's cell, updated in `apply_camera`
     before anything is placed; a change bumps `RenderOrigin::generation`.
  2. Every world-space placement subtracts it in `f64`/`i32` before the cast
     to `f32`: `camera_transform` (eye − origin), `section_transform`
     (pos − origin, exact in `i32`), the crack, `render::apply` (anchor −
     origin), the particle and pickup anchors, `selection`, `weather`,
     `clouds` (its x/z offset), the sky (camera-centred already, unchanged).
     The hand camera and the GUI are camera- or screen-relative and unchanged.
  3. On a generation change, `terrain_frame` rewrites every section entity's
     `Transform` (one pass over at most 8,712 renderers); the other systems
     recompute their transforms every frame anyway.
  4. Meshes stay as they are (sections are section-local, 0..16; entity and
     particle batches are anchor-relative), the simulation is untouched (`f64`),
     and Bevy's frustum culling and fog work in the shifted space unchanged.
  5. Check: the same view at (8, 70, 0), (20,008, 70, −15,000) and
     (1,000,008, 70, 0) pixel-compared (the soak site of WP-4.2 is the middle
     one), plus `testWorld` and the `g` galleries unchanged at the origin.
  About 150 lines in eight modules, most of them owned by the WP-3.x renderers;
  not built in WP-4.9, whose verification budget went to the parity defects.

## 8. Reproducing

Everything is under `build/screens/wp43/` (git-ignored):

| Path | What |
|---|---|
| `scripts/run.sh BIN LABEL {tw,s123,title}` | One scenario on a fresh data dir; `MC_ENV="-u WAYLAND_DISPLAY"` runs under XWayland |
| `scripts/summ.py DIR` | One line per dump: frame, update, phases, rebuilds, storage |
| `scripts/pix.sh BIN LABEL`, `scripts/pngdiff.py A B` | The pixel check |
| `scripts/simprof/` | The simulation sampler (`CARGO_TARGET_DIR=…/target/simprof cargo build --release`; `simprof soak seed-123`) |
| `scripts/meshprof/` | The meshing harness (`Cargo.o1.toml` / `Cargo.o3.toml`) |
| `scripts/appprof/`, `scripts/fold.py` | The client sampler (folded stacks per thread) |
| `bin/` | The binaries measured (`base_dev`, `final_dev`, `final_dist`, `70cdb29_dev`) |
| `m1_*`, `m2_*` | The dumps behind §4 |
