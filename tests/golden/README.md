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

## Tier A scenarios (WP-0.2, WP-0.3)

Common fields: `"name"` (must equal the scenario directory name), `"tier": "A"`,
`"kind"`. Kind-specific fields below. Every trace line, including the last, ends
in `"\n"`. Scenarios run in sorted path order in one JVM, but no scenario's output
depends on another: the oracle reseeds its `Math.random()` replacement stream
(`HarnessMath`, see `tools/oracle/README.md`) from the scenario `seed` (0 when there
is none) at the start of every scenario. A scenario without a `seed` is allowed only
for the kinds whose output is a pure function of the scenario file: `sine_table`,
`nbt_write`, `nbt_read`, `java_hash_order` (`tools/oracle/check-sensitivity.sh`
enforces this).

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
- Optional `"strict": true` calls `java.lang.StrictMath` (fdlibm semantics, the
  same on every JVM) instead; everything else, including the inputs, is
  unchanged. `math/math-sweep-strict` is `math/math-sweep` with this flag.
  On the oracle JVM (OpenJDK 25, x86-64) the two differ in sin 38, cos 28,
  pow 9 (7 by 1 ulp; `pow(-1, ±∞)` is NaN in both, `0x7ff8…` from `Math`,
  `0xfff8…` from `StrictMath`) of 1040 inputs each, and agree on atan2, atan
  and sqrt. PLAN_v4 §3.4 rule 5 decides which one `mc_core::math` follows.
- Accepted function names: `sin`, `cos`, `tan`, `atan`, `sqrt`, `exp`, `log`,
  `log10` (one argument) and `atan2`, `pow` (two). The committed scenarios use
  the first list in the example above.

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

### `kind: "nbt_write"` — vanilla NBT writer

```json
{"name":"primitives","tier":"A","kind":"nbt_write",
 "root":{"type":"compound","name":"Primitives","value":{
   "byte_one":{"type":"byte","value":1},
   "float_one":{"type":"float","value":"0x3f800000"},
   "bytes":{"type":"byte_array","value":"00ff"},
   "names":{"type":"list","value":{"elem":"string","items":["a","b"]}},
   "inv":{"type":"list","value":{"elem":"compound","items":[{"id":{"type":"short","value":1}}]}}}}}
```
Typed tree. A node is `{"type":T,"value":V}`; the root node also has `"name"` and
must be a `compound`. `T` ∈ `byte`, `short`, `int`, `long` (V a JSON integer in
range), `float` (f32 bit-pattern string), `double` (f64 bit-pattern string),
`byte_array` (lower-case hex string), `string`, `list`, `compound` (tag ids 1–10;
`end` = 0 only appears in output). A `list` value is `{"elem":T,"items":[V, …]}`
whose items are bare *values* of type `elem` (a nested list item is again
`{"elem":..,"items":..}`, a compound item is the children object). A `compound`
value is an object of child nodes keyed by name.

The oracle builds the tree with the vanilla classes, children in scenario key order
(`NBTTagCompound.setTag`, NBTTagCompound.java:43; `NBTTagList.setTag`,
NBTTagList.java:50), names the root with `setKey`, and writes it uncompressed with
`CompressedStreamTools.write(root, DataOutputStream)` (CompressedStreamTools.java:47,
which is `NBTBase.writeTag`, NBTBase.java:37-43: type byte, `writeUTF` name, payload).
Trace, four lines:

1. `{"t":0,"e":"nbt_bytes","len":N,"blake3":"<hex>","bytes":"<hex>"}`: the written bytes.
2. `{"t":0,"e":"nbt_tree","root":<canonical tree>}`, taken after the write.
3. `{"t":0,"e":"nbt_roundtrip","equal":<bool>}`: the bytes re-read with
   `CompressedStreamTools.read` (CompressedStreamTools.java:38) and compared as
   canonical trees. When `false`, the line also carries `"reread":<canonical tree>`.
4. `{"t":0,"e":"nbt_gzip","blake3_of_uncompressed_after_gunzip":"<hex>","len":N}`:
   the tree written with `writeGzippedCompoundToOutputStream`
   (CompressedStreamTools.java:27), read back with
   `loadGzippedCompoundFromOutputStream` (:14), re-written uncompressed as in 1;
   digest and length of those bytes. Gzip bytes themselves are never compared
   (PLAN_v4 §4.3, Tier S).

Canonical tree: the input shape, with every compound's children in Java `String`
natural order (UTF-16 code units; only matters for writing, comparison is by parsed
JSON), floats as raw bit patterns (`floatToRawIntBits`), byte arrays as hex. A
list's `elem` is the type it is written with: its first item's type, or, when
empty, the stored `tagType`, which a write sets to 1 (NBTTagList.java:17) and a
read sets from the stream.

Vanilla behaviour the fixtures pin down:

- **Compound order is `HashMap` order** (NBTTagCompound.java:12, :14-23 iterate
  `tagMap.values()`). `nbt_bytes` is byte-exact only for a writer that reproduces
  JDK 8+ `HashMap<String,_>` iteration order for the same insertion order (see
  `java_hash_order`); a sorted-map writer must compare by re-reading.
- **An empty list is written with element type 1 (`TAG_Byte`)** whatever it was
  declared as (NBTTagList.java:14-18).
- **NaN payloads are canonicalised on write**: `DataOutputStream.writeFloat`/
  `writeDouble` use `floatToIntBits`/`doubleToLongBits`, so every NaN is written
  as `0x7fc00000` / `0x7ff8000000000000`. The in-memory tree (`nbt_tree`) keeps
  the scenario bits; `nbt/nan-payloads` therefore has `"equal":false`.
- **Strings are modified UTF-8** (`writeUTF`, NBTTagString.java:21, NBTBase.java:40):
  U+0000 is `c0 80`, a supplementary character is two 3-byte surrogates.
  `nbt/primitives` has both.

Scenarios: `nbt/primitives` (every type, extremes, `-0.0`, NaN, subnormals, 300-byte
array, empty and nested lists, list of compounds, empty compound, non-ASCII key and
strings), `nbt/empty-root` (empty compound named `""`), `nbt/nan-payloads`,
`nbt/level-dat-shape` (`""` → `Data` with the `WorldInfo.updateTagCompound` fields
and types, WorldInfo.java:89-107, plus a `Player` compound shaped by
Entity.java:791-799, EntityLiving.java:557-562, EntityPlayer.java:326-337,
EntityPlayerSP.java:100, InventoryPlayer.java:199-221, ItemStack.java:75-80; children
inserted in that code order).

### `kind: "nbt_read"` — vanilla NBT reader on a file

```json
{"name":"testworld-level-dat","tier":"A","kind":"nbt_read","file":"level.dat","compressed":"gzip"}
```
`file` is a sidecar in the scenario directory (an input; the oracle also writes it
back unchanged so `regen.sh` keeps it). `compressed` ∈ `gzip`
(`loadGzippedCompoundFromOutputStream`, CompressedStreamTools.java:14, as
`SaveHandler` reads `level.dat`) | `none` (`CompressedStreamTools.read`). Trace:

1. `{"t":0,"e":"nbt_file","file":"level.dat","compressed":"gzip","len":414,"blake3":"<hex>","uncompressed_len":504,"uncompressed_blake3":"<hex>"}`:
   the file as stored, and its gunzipped content (for `none` the same bytes twice).
2. `nbt_bytes`, 3. `nbt_tree`, 4. `nbt_roundtrip`, exactly as for `nbt_write`, with
   `nbt_bytes` the tree re-written by `CompressedStreamTools.write` (for
   `testWorld/level.dat` this equals the gunzipped file).

`nbt/testworld-level-dat` reads `level.dat` of the owner's `testWorld` save
(`.reference/mc_b1.7.3-world/testWorld`, not Mojang content).

### `kind: "java_hash_order"` — `java.util.HashSet` iteration order

```json
{"name":"chunkcoordintpair-spiral","tier":"A","kind":"java_hash_order",
 "key_type":"ChunkCoordIntPair","dump_after":[5,13,50,200],
 "ops":[{"op":"add","key":[0,0]},{"op":"remove","key":[1,0]},{"op":"clear"}]}
```
`key_type` ∈ `ChunkCoordIntPair` (key `[x,z]`), `ChunkPosition` and
`ChunkCoordinates` (`[x,y,z]`), `Integer` (a JSON integer in `i32` range), `String`.
The oracle uses the vanilla classes, so `hashCode`/`equals` are vanilla's:

- `ChunkCoordIntPair` (ChunkCoordIntPair.java:16, :13): `chunkXZ2Int(x, z)` =
  `(x < 0 ? 0x80000000 : 0) | (x & 32767) << 16 | (z < 0 ? 0x8000 : 0) | z & 32767`.
  Only 15 bits per axis survive: `(32808,-3)` and `(40,-3)` have the same hash.
- `ChunkPosition` (ChunkPosition.java:23): `x * 8976890 + y * 981131 + z` (wrapping).
- `ChunkCoordinates` (ChunkCoordinates.java:32-34): `this.x + this.z << 8 + this.y << 16`,
  which Java parses as `((x + z) << (8 + y)) << 16`, shift distances mod 32. The
  low 16 bits are always zero; for most `y` all keys collide. No vanilla hash
  collection holds `ChunkCoordinates`; the scenario is mainly the hash sample.
- `Integer`: the value. `String`: `String.hashCode` over UTF-16 code units.

`ops` run in order on one `new HashSet()` (default capacity 16, load factor 0.75,
as every vanilla `new HashSet()`): `add`, `remove` (no-ops when already present /
absent), `clear` (keeps the table size, as `HashMap.clear` does). Trace:

1. `{"t":0,"e":"hash_codes","key_type":"..","samples":[{"key":<key>,"hash":<i32>}, …]}`:
   the first 32 distinct keys of `ops` (adds and removes) in op order, with their
   `hashCode()`.
2. For each `n` in `dump_after` (strictly increasing, 1..len(ops)) and after the
   last op (once, even if listed): `{"t":0,"e":"hash_order","after":n,"size":<size>,"hashes":[<i32>, …],"keys":[<key>, …]}`
   after the first `n` ops, both arrays in iteration order.

**Parity target: the JDK 8+ `HashMap`** (the oracle JDK; Prism runs the vanilla
client on a modern JDK, so that is the owner's default). b1.7.3 on Java 6/7 iterated
differently (other hash spreading, head insertion, order reversal on resize). The
algorithm, as the oracle's built-in model checks it at every dump:

- `spread(h) = h ^ (h >>> 16)`; bucket = `spread(h) & (capacity - 1)`; the table
  is allocated at capacity 16 on the first add; capacity doubles when the size
  exceeds `0.75 * capacity`, and also when an add appends to a bucket that already
  holds 8 entries while capacity < 64 (`treeifyBin` resizes instead of treeifying).
- Iteration: buckets in index order; within a list bucket, insertion order (new
  entries go to the tail; a resize splits each bucket into `i` and `i + oldCap`
  keeping relative order; a removed and re-added key goes to the tail).
- **Tree bins:** an add that finds 8 entries in its bucket at capacity ≥ 64
  converts the bucket to a red-black tree (`TreeNode`); a later resize split turns
  a half with at most 6 entries (`UNTREEIFY_THRESHOLD`) back into a list, keeping
  the tree's order, and `clear` empties all buckets. A tree bin's iteration order
  is the tree's `next` linkage (`treeify`, `putTreeVal`, `moveRootToFront`,
  `split`, `removeTreeNode`), not insertion order. **Vanilla gameplay hits this**:
  `World.positionsToUpdate` (World.java:1868-1885, a 19×19 square of
  `ChunkCoordIntPair`, capacity 512) puts up to 19 keys in one bucket, because the
  bucket is roughly `(x ^ z) & 511`. The oracle checks tree buckets for placement and
  membership only. Within a tree, keys order by hash, then `compareTo` for
  `Comparable<Self>` keys (`String`, `Integer`), then `System.identityHashCode`;
  none of the vanilla key classes is `Comparable<Self>`, so two distinct keys with
  equal hash in one tree bin would make the order irreproducible. The oracle
  refuses such scenarios.

Scenarios (tree bins created, final capacity): `hash/chunkcoordintpair-spiral`
(16, 512), `hash/chunkcoordintpair-active-chunks` (the `positionsToUpdate` pattern
for three player positions with `clear` between them; 48, 512),
`hash/chunkposition-explosion` (an `Explosion.destroyedBlockPositions`-like ball
plus redstone-wire neighbours, Explosion.java:65, BlockRedstoneWire.java:161; 0,
256), `hash/chunkcoordinates` (0, 64), `hash/integer-chunk-keys` (`chunkXZ2Int`
values as in `ChunkProvider.droppedChunksSet`, ChunkProvider.java:12; 4, 256),
`hash/string-names` (block and item names, incl. `"Aa"`/`"BB"`, equal hash 2112;
0, 256).
