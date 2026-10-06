# ADR-007 — Golden harness: producing ground truth from the Java build

**Status:** proposed (session-zero)
**Blocks:** all transcription work (start here)
**Prior art (added after source inspection):** MinecraftOSS (this repo,
`third_party/minecraftoss`, commit `4013a68`) independently built this exact
design for Java 26.3 — ~25k LOC of `bin/check_*_trace.rs` binaries replaying
`scenarios/*.json`, with ground truth dumped by a Java-side harness into
artifact catalogs (e.g. `artifacts/entity-catalog/26.3.json`, consumed by
`core/src/entity_data.rs`). Adopt their binary-per-behavior layout alongside
scenario files, and their catalog-dump technique for Tier C entity defaults.
Their Java-side artifacts are not in this copy — building ours remains
ADR-007's core deliverable. See blueprint Part V for the full assessment.

---

## 1. Context

The rewrite's bottleneck is not code volume — it is **proving behavioral parity**.
The Java codebase is a complete, executable specification. This ADR defines the
machinery that turns it into checkable artifacts:

> One **scenario file** drives **both engines**. The Java build produces
> `expected.jsonl`; the Rust workspace replays the same scenario and produces
> `actual.jsonl`. CI asserts equality. No parity claim exists without a fixture.

```
                        scenario.json  (single source of truth, checked in)
                       ┌───────────────────────────────┐
                       │                               │
            ┌──────────▼──────────┐         ┌──────────▼──────────┐
            │  Java harness       │         │  Rust runner        │
            │  src/harness/java   │         │  mc-golden (dev)    │
            │  (new source set,   │         │  (drives mc-worldgen│
            │   runs game logic   │         │   /world/entity     │
            │   headless on JVM)  │         │   headless)         │
            └──────────┬──────────┘         └──────────┬──────────┘
                       │                               │
                       ▼                               ▼
                expected.jsonl  ────── diff ──────  actual.jsonl
                expected.blake3                   (CI: exact match)
                screenshots/*.png                 (CI: phash ≤ τ)
```

## 2. Why a harness source set (and not the web build for everything)

Measured facts that shape this:

- The game imports **`net.PeytonPlayz585.opengl.GL11`** (this repo's TeaVM/WebGL
  shim), not `org.lwjgl.opengl.GL11`. The real LWJGL jars in `jars/` are
  vestigial — there is **no runnable desktop client**. Only the web build runs.
- Gameplay logic (`ChunkProviderGenerate`, `World`, `Chunk`, `Entity`,
  `Packet*`, `CompressedStreamTools`) has **no GL/JSO reachability** from a
  direct constructor path. `World.spawnParticle` fans out through
  `worldAccesses: ArrayList<IWorldAccess>` (World.java:875) — a clean seam for
  a no-op listener.
- Therefore: logic tiers run on a **plain JVM** via a new `src/harness/java`
  source set compiled against the existing `src/main/java` (+ `src/lwjgl/java`
  on the classpath; its JSO annotations are inert unless a browser method is
  actually *called*, which logic paths never do). Only the full-client tier
  needs a browser.

**Zero game-code diffs** is a hard rule (Tier D adds one isolated export file;
see §4).

## 3. Tiers

| Tier | Runs | Unlocks | Java entry points | Size est. |
|---|---|---|---|---|
| **A** | JVM, logic only | `mc-core`, `mc-worldgen`, `mc-nbt` (packet codecs: Phase 2 — MP gate) | `ChunkProviderGenerate.provideChunk`, `ChunkProviderSky/Hell`, `CompressedStreamTools` | ~400 LOC |
| **B** | JVM, logic only | `mc-world`, `mc-blocks` (fluids/fire/saplings/redstone/pistons), lighting | `WorldClient` (client-authoritative port — no server classes exist; `WorldClient extends World`) + no-op `IWorldAccess`, manual `worldTime`, `tick()` ×N | ~400 LOC |
| **C** | JVM, logic only | `mc-entity` (physics, AI, projectiles, minecarts) | Tier B world + `EntityList.createEntityByName`, scripted `moveEntity`/`onUpdate` | ~300 LOC |
| **D** | headless Chromium | `mc-render`, `mc-ui`, end-to-end feel | TeaVM web build + exposed JS control API (§4.4) | ~300 LOC JS/TS + export file |

Build order: **A first** (unblocks the biggest, most mechanical transcription
immediately), B while worldgen transcription runs, C with entity work, D when
rendering starts.

## 4. Java-side design

### 4.1 Gradle wiring (only change to the build)

```groovy
// build.gradle — additive only
sourceSets {
    harness { java { srcDir 'src/harness/java' } }
}
tasks.register('goldenHarness', JavaExec) {
    mainClass = 'harness.Main'
    classpath = sourceSets.harness.runtimeClasspath
    args 'tests/golden', 'build/golden-out'   // scenarios in, artifacts out
}
```

`src/harness/java/harness/Main.java` reads every
`tests/golden/<area>/<name>/scenario.json`, dispatches on scenario `tier`, and
writes `expected.jsonl` next to it. Scenario parsing can reuse the bundled
parser (`net/minecraft/src/J_SajParser.java`); the writer is hand-rolled so
floats serialize as bit patterns (§5.2).

### 4.2 Tier sketch

```java
// Tier A — terrain
WorldClient w = HarnessWorld.headless(seed);            // client-authoritative: WorldClient extends World
Chunk c = w.getChunkProvider().provideChunk(cx, cz);
dump.chunkDigest(cx, cz, c.blocks);                     // blake3 over bytes
dump.chunkBlocks(cx, cz, c.blocks);                     // full copy, opt-in
dump.heightMap(c); dump.biomeArray(c);

// Tier B — world ticks under script
for (int t = 0; t < sc.ticks; t++) {
    sc.applyOpsAtTick(t, w);                            // set_block, set_time, …
    w.tick();
    sc.dumpAtTick(t, w, dump);                          // chunk_digest, light_at, …
}

// Tier C — entities
Entity e = EntityList.createEntityByName(kind, w);
HarnessRand.seedEntity(e);                              // §4.3
sc.applyEntityOps(t, e);                                // set_motion, punch, …
e.onUpdate();
dump.entityState(e);                                    // pos/motion/state bits
```

### 4.3 Determinism discipline (the make-or-break section)

Census of nondeterminism in the Java code — verified in source:

| Site | Line | Problem | Harness rule |
|---|---|---|---|
| `Entity.rand = new Random()` | Entity.java:49 | **unseeded** — per-run AI/projectile jitter differs | After construction, reflectively `setSeed(hash(worldSeed, entityId))` on every entity (`HarnessRand.seedEntity`) |
| `World.rand = new Random()` | World.java:81,122,168 | **unseeded** — drives random block ticks, mob spawns | Set immediately after world construction from `worldSeed` |
| `World.field_9437_g = (new Random()).nextInt()` | World.java:74,115,161 | **clock-seeded** auxiliary rand | Overwrite after construction (reflection), same derivation |
| `Chunk` chunk rand | Chunk.java:593 | Already deterministic: `seed + x²·4987142 + x·5947611 + z²·4392871 + z·389711 ^ extra` | Nothing — transcribe formula verbatim into Rust |
| `System.currentTimeMillis` paths | scattered (menus, day-cycle auto) | Wall clock | Harness never enters those paths; `worldTime` set via ops only |
| Threading | `ThreadDownloadImage`-style helpers | Async | Never reachable: no render/client objects constructed |

Rust consequence (already mandated by blueprint Part 0): the Rust port must
**also** own seeded-per-entity RNGs, not one global — the trace comparison will
catch any slip.

### 4.4 Tier D — full client in headless Chromium

1. Add **one file** to the game sources: `GoldenExports.java` exposing
   `@JSBody`-exported static hooks (`startScenario(seedJson)`, `applyOp(t, op)`,
   `dumpState()`, `screenshot()`) calling into a small scenario interpreter that
   drives `Minecraft`'s own loop. Guarded by a query-param/flag so production
   builds are inert. This is the single sanctioned game-code diff.
2. Runner: Playwright script loads the built `web/index.html`, executes the
   scenario, collects `dumpState()` JSONL + canvas PNGs.
3. Render fixtures live under `tests/golden/render/*` and compare by perceptual
   hash (threshold τ tuned once; store hash + PNG).

## 5. Artifact formats

### 5.1 Scenario (`scenario.json`) — reviewed like code

```json
{
  "name": "piston-push-corner-case",
  "tier": "B",
  "seed": -4172144991579068934,
  "dimension": "overworld",
  "ticks": 200,
  "ops": [
    { "t": 0,  "op": "set_block", "x": 0,  "y": 70, "z": 0, "id": 33, "meta": 1 },
    { "t": 5,  "op": "pulse_redstone", "x": -1, "y": 70, "z": 0 },
    { "t": 10, "op": "spawn", "kind": "Minecart", "pos": [0.5, 71, 0.5] }
  ],
  "dumps": [
    { "every": 1, "what": "entity_state", "which": "all" },
    { "t": 199, "what": "chunk_digest", "x": 0, "z": 0 }
  ]
}
```

Agents write scenarios in PRs; humans review the *scenario*, not the trace.

### 5.2 Trace (`expected.jsonl` / `actual.jsonl`)

One JSON object per line:

```json
{"t":199,"e":"chunk_digest","x":0,"z":0,"blake3":"9f2b…"}
{"t":199,"e":"entity","id":12,"kind":"Minecart",
 "posX":"0x4093c00000000000","posY":"0x4051c00000000000","posZ":"0x3ff0000000000000",
 "motionY":"0xbfd999999999999a","yaw":"0x42b4cccd","flags":"on_ground|collided_h",
 "state":{"cargo":"rideable","damageTaken":"0x00000000"}}
```

Two non-negotiables:

1. **All floats serialize as IEEE-754 bit patterns** (`u64`/`u32` hex). Java
   `Double.toString` and Rust `{}` produce different shortest-repr strings;
   bits are unambiguous and parse identically everywhere. Human-readable values
   may appear in a parallel `"dbg"` field, never used for comparison.
2. **Digests are blake3** over canonical byte arrays (chunk blocks, heightmap,
   light nibble arrays, packet bytes). Full arrays are opt-in (`"full": true`)
   for first-failure debugging — the diff tool renders a 16×128×16 slice view
   from them.

### 5.3 Packet fixtures — **gated behind multiplayer (Phase 2)**

Not built while the goal is singleplayer parity. When the MP gate opens:
`Packet*.writePacketData(DataOutputStream)` with fixed field values → exact
bytes → blake3 + full hex. Also golden NBT: encode fixed structures via
`CompressedStreamTools`, plus round-trip any real `.dat` files committed under
`tests/golden/nbt/`. Live-server interop remains a separate manual gate
(ADR-004). *(NBT fixtures above are ungated — they serve SP save/load.)*

## 6. Known parity hazards the traces will surface (policy set now)

| Hazard | Site | Policy |
|---|---|---|
| `HashMap` iteration order | `Chunk.chunkTileEntityMap` (Chunk.java:21) — tile-entity tick order affects piston/redstone chains | Rust port replicates Java HashMap bucket order for **this map only** (port the structure; insertion-seeded). Flag any other gameplay-ordered `HashMap` found during transcription — do not silently swap in `BTreeMap`. |
| Entity-list order | `World.loadedEntityList` (ArrayList) — preserved by construction in Rust; dumps include list index | Dump `idx` for every entity |
| Transcendental drift | JVM `Math.sin` intrinsic vs Rust `f64::sin` may differ in last ULP | All such calls go through `mc_core::math` (blueprint Part 0 §4); Tier C includes a dedicated `math_parity` fixture asserting bit-equality across a sweep — deviations get a wrapped Java-exact implementation |
| Unseeded RNGs | §4.3 census | Seeding protocol + reflective audit that fails if a `Random` field is discovered unseeded |

## 7. Rust side

`mc-golden` — test-support crate, `dev-dependency` of each logic crate's tests:

```rust
// mc-worldgen/tests/golden.rs
#[test]
fn overworld_seed_12345() { mc_golden::run("tests/golden/worldgen/seed-12345"); }
```

`mc_golden::run(dir)`: load `scenario.json` → execute via the crate's headless
API (same functions `mc-app` uses, no bevy) → emit `actual.jsonl` to
`target/golden/` → byte-compare against `expected.jsonl` → on mismatch print a
unified diff with `dbg`-field annotation and, for digests, an automatic re-run
with `full: true` to locate the first differing block/tick. Exit code drives CI.

Screenshot tests (render crates) use the same runner with the phash comparator.

## 8. CI policy

- **Fixtures are checked in.** PR CI runs only Rust comparison — fast, no JDK.
- `golden-regen` workflow (manual/weekly): rebuilds fixtures from Java in a
  Temurin container; opens a PR if digests drift. Fixture drift without a Java
  source change = determinism bug in the harness — investigate, never force-push.
- Scenario coverage budget per crate (from blueprint Part III) is tracked in
  `tests/golden/COVERAGE.md` — PRs that transcribe behavior must add or cite a
  scenario covering it (`needs-golden` tag otherwise).

## 9. Acceptance criteria for the harness itself

1. Tier A regenerates 5 seeds × 4 chunk corners × 3 dimensions reproducibly
   (byte-identical across 3 runs, 2 machines).
2. Deliberate perturbation test: flipping one bit in any seeded RNG, or swapping
   two tile-entity insertions, **fails** the corresponding fixture (proves the
   traces are actually sensitive — a harness that passes everything proves
   nothing).
3. `mc_golden::run` failure output is actionable enough that an agent, given
   only the diff, identifies the diverging Java source line in one attempt
   (validated once by hand during Tier A bring-up).
4. Full Tier A regen < 60s; Tier B/C scenarios stay under 10 min each on CI
   runners; Tier D excluded from per-PR CI.

## 10. Explicitly out of scope

- Render-pipeline internals (→ ADR-002), integrated-server threading (→
  ADR-004), redstone update-graph design (→ ADR-006) — this ADR only *observes*
  their behavior, it does not design them.
- Perf: the harness runs unoptimized Java and exact-comparison Rust; speed is
  not a fixture goal.
