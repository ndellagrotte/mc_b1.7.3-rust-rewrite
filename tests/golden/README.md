# Golden fixtures (PLAN_v4 §4, ADR-007 §5)

One `scenario.json` drives both engines. The Java oracle (`tools/oracle/`)
writes `expected.jsonl`; the Rust runner (`crates/mc-golden`) replays the same
scenario and compares. Nobody edits an `expected.jsonl` by hand; regenerate it
with `tools/oracle/regen.sh`.

Layout: `tests/golden/<area>/<name>/scenario.json` + `expected.jsonl`
(+ optional sidecar binaries named in the trace).

## Encoding rules (both sides, non-negotiable)

- Every `float`/`double` is a lower-case hex IEEE-754 bit pattern string:
  `f32` → `"0x" + 8 hex digits`, `f64` → `"0x" + 16 hex digits`. Never a
  decimal float. A parallel `"dbg"` field may carry the decimal form; it is
  never compared.
- `int`/`long`/`boolean`/strings are plain JSON values (`i64` fits a JSON
  number on both sides; no hex for integers).
- Byte arrays: `blake3` (32-byte, hex, lower-case) over the raw bytes; full
  arrays, when requested, are lower-case hex strings of the raw bytes.
- Comparison is per line, by parsed JSON value (key order does not matter;
  extra or missing keys do), in file order. The first differing line and field
  is reported with floats decoded.
- Every trace line has `"t"` (tick; `0` for Tier A) and `"e"` (event name).
  Lines of one scenario are emitted in a fixed, documented order.

## Tier A scenarios (WP-0.2)

Common fields: `"name"` (must equal the scenario directory name), `"tier": "A"`,
`"kind"`. Kind-specific fields below. Every trace line, including the last, ends
in `"\n"`. Scenarios run in sorted path order in one JVM, but no scenario's output
depends on another: the oracle reseeds its `Math.random()` replacement stream
(`HarnessMath`, see `tools/oracle/README.md`) from the scenario `seed` (0 when there
is none) at the start of every scenario.

### `kind: "java_random"` — `java.util.Random` sequences

```json
{"name":"java-random-123","tier":"A","kind":"java_random","seed":123,
 "draws":[{"call":"nextInt","n":64},{"call":"nextInt","bound":10,"n":64},
          {"call":"nextLong","n":64},{"call":"nextFloat","n":64},
          {"call":"nextDouble","n":64},{"call":"nextGaussian","n":64},
          {"call":"nextBoolean","n":64}]}
```
One `new Random(seed)`; the draws run in list order on that single instance.
Trace, one line per draw:
`{"t":0,"e":"rng","i":<running index from 0>,"call":"nextInt","bound":10,"v":7}`
(`bound` present only for bounded `nextInt`, which calls `nextInt(bound)`; `bound`
on any other call is an error). `v` is a JSON number for
`nextInt`/`nextLong`, a bit-pattern string for `nextFloat`/`nextDouble`/
`nextGaussian`, a JSON boolean for `nextBoolean`.

### `kind: "sine_table"` — `MathHelper.SIN_TABLE`

```json
{"name":"sine-table","tier":"A","kind":"sine_table"}
```
Trace: `{"t":0,"e":"sine_table","len":65536,"blake3":"<hex>","file":"sine_table.f32le.bin"}`.
`file` is relative to the scenario directory; the sidecar is committed next to
`expected.jsonl`.
Sidecar `sine_table.f32le.bin`: the 65536 floats as little-endian `f32`, in
index order (262144 bytes). The digest is over exactly those bytes.

### `kind: "math_sweep"` — JVM `Math.*` bit patterns

```json
{"name":"math-sweep","tier":"A","kind":"math_sweep","seed":20261006,"count":1024,
 "functions":["sin","cos","atan2","atan","pow","sqrt"]}
```
Inputs: for each function in list order, the fixed edge-case list first
(agent-defined, documented in the scenario under `"edge_cases"` as bit-pattern
strings so both sides read them from the file), then `count` pseudo-random
inputs from `new Random(seed)` using `nextDouble() * 2000 - 1000`
(two-argument functions draw `x` then `y`). The same `Random` instance runs
across all functions in order. Trace, one line per input:
`{"t":0,"e":"math","fn":"atan2","i":<index within fn>,"x":"0x…","y":"0x…","v":"0x…"}`
(`y` only for two-argument functions). All values `f64`.

- `edge_cases` is an object keyed by function name; every listed function has an
  entry (possibly `[]`). One-argument functions (`sin`, `cos`, `atan`, `sqrt`)
  list bit-pattern strings; two-argument functions (`atan2`, `pow`) list
  `[x, y]` pairs of bit-pattern strings.
- `i` counts from 0 per function over the edge cases first, then continues
  through the `count` random inputs (so random input k has `i = edge_count + k`).
- Argument order is `(x, y)` = Java's (first, second) parameter:
  `v = Math.atan2(x, y)` (Java names the first parameter `y`; in Rust this is
  `x.atan2(y)`) and `v = Math.pow(x, y)` (`x.powf(y)`).
- `java.lang.Math`, not `StrictMath`, because that is what vanilla calls. The JDK
  only promises 1-ulp accuracy for these, so the bit patterns are those of the
  oracle JVM (HotSpot intrinsics); they already differ from glibc `libm` in
  about 180 of the 6240 inputs, mostly `atan2`, by 1 ulp.

### `kind: "terrain"` — chunk generation and population

```json
{"name":"overworld-seed-123","tier":"A","kind":"terrain","seed":123,
 "dimension":"overworld",
 "chunks":[{"x":0,"z":0,"full":true},{"x":-3,"z":7},{"x":100,"z":-50}]}
```
`dimension` ∈ `overworld` | `nether`. World: `new World(new SaveHandlerMP(),
"harness", provider, seed)` with `WorldProviderSurface` / `WorldProviderHell`;
`world.rand.setSeed(seed)` right after construction (§4.2). The oracle also
overwrites the constructor's other unseeded state, from `d = new Random(seed)`:
`field_9437_g = d.nextInt()` (World.java:75, clock-seeded random-tick LCG state),
then `soundCounter = d.nextInt(12000)` (World.java:90, drawn from the unseeded
`rand`). Neither reaches a Tier A trace; Tier B must replicate them. The
`World(ISaveHandler, String, WorldProvider, long)` constructor loads no chunks
and does not search for a spawn point. For each chunk in list order:

1. **generated**: `chunk = world.getChunkProvider()`'s underlying generator
   `provideChunk(x, z)` result *before* population (obtain via
   `world.getChunkFromChunkCoords(x, z)` while no neighbour is loaded; the
   wrapper `ChunkProvider` populates only when x+1/z+1 neighbours exist).
   Emit `chunk_digest` with `"stage":"generated"`. Precisely: the state right
   after `world.getChunkFromChunkCoords(x, z)` first returns. If the target or
   any of its 8 neighbours was already loaded by an earlier chunk entry, that
   state can include population (the target's own, or a neighbour's writes);
   the oracle warns on stderr but still emits it, deterministically. The
   committed scenarios use chunks far apart, so their `generated` stage is pure
   generator output. Note the nether generator computes no height map or sky
   light (ChunkProviderHell.java:180-188), so those arrays are all zero there.
2. **populated**: load neighbours `(x+dx, z+dz)` for `dz in -1..=1`, then
   `dx in -1..=1` (dz outer, dx inner), via `world.getChunkFromChunkCoords`;
   assert the target `isTerrainPopulated` (the oracle fails otherwise). Emit
   `chunk_digest` with `"stage":"populated"`. With this order the population
   calls run (x-1,z-1), (x,z-1), (x-1,z), then the target, triggered by loading
   (x-1,z), (x+1,z), (x,z+1) and (x+1,z+1) respectively (ChunkProvider.java:52-66);
   a populator writing outside the 3×3 may load and generate further chunks.
   No world tick runs, so queued light updates (`World.lightingToUpdate`) are
   never processed: block light is still all zero and sky light reflects only
   the in-chunk updates `Chunk` does itself.
3. **biome**: overworld only. `WorldChunkManager.loadBlockGeneratorData(null,
   x*16, z*16, 16, 16)`: emit `{"t":0,"e":"biome","x":..,"z":..,
   "biomes":"<blake3 over the 256 biomeName strings joined with '\n', UTF-8>",
   "temperature":"<blake3 over 256 f64 LE>","humidity":"<blake3 over 256 f64 LE>"}`.
   Array order is the Java array order (index `i*16 + j` as the Java fills it).
   `x`, `z` are the chunk coordinates. The oracle calls this on a fresh
   `new WorldChunkManager(world)` (same output as the world's own instance,
   the noise being a pure function of seed and coordinates, but without
   touching its cached arrays) and hashes exactly the first 256 entries of
   `temperature` and `humidity`.

`chunk_digest` line:
`{"t":0,"e":"chunk_digest","x":0,"z":0,"stage":"generated","blocks":"<blake3>","data":"<blake3>","skylight":"<blake3>","blocklight":"<blake3>","heightmap":"<blake3>"}`
over `Chunk.blocks` (32768 bytes), `data.data`, `skylightMap.data`,
`blocklightMap.data` (16384 bytes each) and `heightMap` (256 bytes).
When the chunk has `"full": true`, each `chunk_digest` line is followed by
`{"t":0,"e":"chunk_full","x":..,"z":..,"stage":..,"blocks":"<hex>","data":"<hex>","skylight":"<hex>","blocklight":"<hex>","heightmap":"<hex>"}`.

Chunks are processed strictly in list order on one world, so a later chunk's
neighbour loading may already have happened; this is part of the fixture.
Vanilla quirk this exposes: `ChunkProviderHell.populate` (ChunkProviderHell.java:302)
never reseeds `hellRNG`; it continues from whatever the last `provideChunk`
(seeded from chunk coordinates only, not the world seed) and the populations
since left behind. Nether population is therefore load-order dependent, and the
Rust port must reproduce the exact chunk load order. Overworld population
reseeds per chunk (world seed and coordinates), but the blocks it leaves in a
target chunk still depend on the order in which overlapping neighbours populate.
