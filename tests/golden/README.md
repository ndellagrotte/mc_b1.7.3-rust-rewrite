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

Common fields: `"name"`, `"tier": "A"`, `"kind"`. Kind-specific fields:

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
(`bound` present only for bounded `nextInt`). `v` is a JSON number for
`nextInt`/`nextLong`, a bit-pattern string for `nextFloat`/`nextDouble`/
`nextGaussian`, a JSON boolean for `nextBoolean`.

### `kind: "sine_table"` — `MathHelper.SIN_TABLE`

```json
{"name":"sine-table","tier":"A","kind":"sine_table"}
```
Trace: `{"t":0,"e":"sine_table","len":65536,"blake3":"<hex>","file":"sine_table.f32le.bin"}`.
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

### `kind: "terrain"` — chunk generation and population

```json
{"name":"overworld-seed-123","tier":"A","kind":"terrain","seed":123,
 "dimension":"overworld",
 "chunks":[{"x":0,"z":0,"full":true},{"x":-3,"z":7},{"x":100,"z":-50}]}
```
`dimension` ∈ `overworld` | `nether`. World: `new World(new SaveHandlerMP(),
"harness", provider, seed)` with `WorldProviderSurface` / `WorldProviderHell`;
`world.rand.setSeed(seed)` right after construction (§4.2). For each chunk in
list order:

1. **generated**: `chunk = world.getChunkProvider()`'s underlying generator
   `provideChunk(x, z)` result *before* population (obtain via
   `world.getChunkFromChunkCoords(x, z)` while no neighbour is loaded; the
   wrapper `ChunkProvider` populates only when x+1/z+1 neighbours exist).
   Emit `chunk_digest` with `"stage":"generated"`.
2. **populated**: load neighbours `(x+dx, z+dz)` for `dz in -1..=1`, then
   `dx in -1..=1` (dz outer, dx inner), via `world.getChunkFromChunkCoords`;
   assert the target `isTerrainPopulated`. Emit `chunk_digest` with
   `"stage":"populated"`.
3. **biome**: overworld only. `WorldChunkManager.loadBlockGeneratorData(null,
   x*16, z*16, 16, 16)`: emit `{"t":0,"e":"biome","x":..,"z":..,
   "biomes":"<blake3 over the 256 biomeName strings joined with '\n', UTF-8>",
   "temperature":"<blake3 over 256 f64 LE>","humidity":"<blake3 over 256 f64 LE>"}`.
   Array order is the Java array order (index `i*16 + j` as the Java fills it).

`chunk_digest` line:
`{"t":0,"e":"chunk_digest","x":0,"z":0,"stage":"generated","blocks":"<blake3>","data":"<blake3>","skylight":"<blake3>","blocklight":"<blake3>","heightmap":"<blake3>"}`
over `Chunk.blocks` (32768 bytes), `data.data`, `skylightMap.data`,
`blocklightMap.data` (16384 bytes each) and `heightMap` (256 bytes).
When the chunk has `"full": true`, each `chunk_digest` line is followed by
`{"t":0,"e":"chunk_full","x":..,"z":..,"stage":..,"blocks":"<hex>","data":"<hex>","skylight":"<hex>","blocklight":"<hex>","heightmap":"<hex>"}`.

Chunks are processed strictly in list order on one world, so a later chunk's
neighbour loading may already have happened; this is part of the fixture.
