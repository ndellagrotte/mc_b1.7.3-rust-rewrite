# Golden fixtures (PLAN_v4 §4, ADR-007 §5)

One `scenario.json` drives both engines. The Java oracle (`tools/oracle/`)
writes `expected.jsonl`; the Rust runner (`crates/mc-golden`) replays the same
scenario and compares. Nobody edits an `expected.jsonl` by hand; regenerate it
with `tools/oracle/regen.sh`.

Layout: `tests/golden/<area>/<name>/scenario.json` + `expected.jsonl`
(+ optional sidecar binaries named in the trace). The long-run fixtures under
`tests/golden/soak/` are left out of the default `regen.sh` and check-script runs and of
`cargo test`; see "Soak" at the end.

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

## Tier A scenarios (WP-0.2, WP-0.3, WP-1.1)

Common fields: `"name"` (must equal the scenario directory name), `"tier": "A"`,
`"kind"`. Kind-specific fields below. Every trace line, including the last, ends
in `"\n"`. Scenarios run in sorted path order in one JVM, but no scenario's output
depends on another: the oracle reseeds its `Math.random()` replacement stream
(`HarnessMath`, see `tools/oracle/README.md`) from the scenario `seed` (0 when there
is none) at the start of every scenario. A scenario without a `seed` is allowed only
for the kinds whose output is a pure function of the scenario file: `sine_table`,
`nbt_write`, `nbt_read`, `java_hash_order`, `block_table`, `item_table`, `recipe_table`,
`stat_table`
(`tools/oracle/check-sensitivity.sh` enforces this).

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
  only promises 1-ulp accuracy for these; the oracle therefore runs with HotSpot's
  transcendental intrinsics disabled (`tools/oracle/env.sh`, PLAN_v4 §3.4 rule 5,
  decided in WP-0.3), which makes `Math` return exactly the `StrictMath` (fdlibm)
  bits on every JVM.
- Optional `"strict": true` calls `java.lang.StrictMath` (fdlibm semantics, the
  same on every JVM) instead; everything else, including the inputs, is
  unchanged. `math/math-sweep-strict` is `math/math-sweep` with this flag.
  With the oracle's JVM flags the two fixtures are byte-identical. (With the
  intrinsics on, OpenJDK 25 on x86-64 differed in sin 38, cos 28, pow 9 of 1040
  inputs each, incl. the NaN sign of `pow(-1, ±∞)`; that is the measurement behind
  the rule 5 decision.)
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

Committed terrain scenarios: three chunks × three seeds × both dimensions
(`<dimension>-seed-<seed>`), plus `<dimension>-seed-42-spread` (WP-1.4): 14
chunks at least four apart (so every `generated` stage is pure generator output
and no two share a `chunkXZ2Int` key), up to ±1874999, including chunk
`(784426, 0)`, where the density noise's `(int)` casts saturate (the Far
Lands), dumped in full. WP-1.7 adds `overworld-seed-123-dungeon` (chunk (1, 1),
whose own population places a dungeon: spawner at (31, 13, 28), chests at
(29, 13, 28) and (30, 13, 30); dumped in full; the loot and the mob are saved by
`tick/dungeon-chest-save`) and the pair `nether-seed-123-order-ab` /
`nether-seed-123-order-ba` (chunks (0, 0) and (2, 0), whose 3×3 neighbourhoods
share a column, in both orders: the `hellRNG` quirk above makes chunk (0, 0)'s
populated digest differ between them). Rust side:
`crates/mc-golden/src/terrain_driver.rs`, run by
`crates/mc-sim/tests/golden_worldgen.rs`, which compares every line.

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

### `kind: "block_table"` — the block registry after static init (WP-1.1)

```json
{"name":"block-table","tier":"A","kind":"block_table"}
```
No `seed` (the output is a pure function of the vanilla classes). The oracle
(`tools/oracle/java/harness/BlockTable.java`) loads the vanilla classes into a
**fresh class loader**, so the dump is the state right after `Block`'s static
initialiser (Block.java:592-706, which also initialises `Material`, `MapColor`,
`Item` and `StatList`) whatever ran earlier in the JVM, and reads everything by
reflection. `BlockLeavesBase.graphicsLevel` keeps its default (false; no renderer).
Trace, in this order:

1. 256 lines `{"t":0,"e":"block_def","id":N,"present":false}` or, when
   `Block.blocksList[N] != null`:
   `{"t":0,"e":"block_def","id":N,"present":true,"class":"<simple class name>","fields":{…},`
   `"isOpaqueCube":b,"renderAsNormalBlock":b,"getRenderType":i,"getRenderBlockPass":i,`
   `"isCollidable":b,"getMobilityFlag":i,"tickRate":i,"getBlockName":s|null,`
   `"damageDropped":[16],"idDropped":[16],"quantityDropped":i,`
   `"getBlockTextureFromSide":[6],"getBlockTextureFromSideAndMetadata":[96]}`.
   The event is `block_def`, not `block`, which is the Tier B block dump.
   - `fields`: every non-static field declared from `Block` down to the instance's
     class (private and protected included), keyed by Java field name (a name
     repeated in one class chain is an oracle error).
   - Method results are the virtual calls on the instance (protected
     `damageDropped` included): `damageDropped(m)` and `idDropped(m, new Random(0))`
     for `m` in 0..16; `quantityDropped(new Random(0))`; **a fresh `new Random(0)`
     per call**, so the gravel/tall-grass/glowstone/leaves/ore draws are the first
     draw of seed 0; `getBlockTextureFromSide(s)` for `s` in 0..6;
     `getBlockTextureFromSideAndMetadata(s, m)` at index `s * 16 + m`.
   - A call that throws is `{"throws":"<exception simple name>"}`. The one case:
     `BlockPistonExtension.getBlockTextureFromSideAndMetadata` with `m & 7` of 6 or 7
     on a side other than `m & 7` throws `ArrayIndexOutOfBoundsException`
     (`faceToSide` has 6 entries, BlockPistonExtension.java:50; vanilla bug, 24
     entries of id 34). The Rust port returns `None` there.
2. `{"t":0,"e":"block_statics",…}` with the per-id arrays of Block.java:17-23 under
   their Java names (`tickOnLoad`, `opaqueCubeLookup`, `isBlockContainer`,
   `lightOpacity`, `canBlockGrass`, `lightValue`, `setNeighborNotifyOnMetadataChange`
   — later MCP's `requiresSelfNotify`), each 256 entries; `blockName` (256,
   `getBlockName()` or null); `fire_encourage` / `fire_catch`: the fire singleton's
   `chanceToEncourageFire` / `abilityToCatchFire` (BlockFire.java:6-7, filled by
   `initializeBlock`, Block.java:700). The two arrays also appear in block 51's
   `fields`.
3. One `{"t":0,"e":"material","name":"<static field name in Material.java>","class":…,`
   `"fields":{…},"getIsLiquid":b,"isSolid":b,"getCanBlockGrass":b,"getIsSolid":b,`
   `"getBurning":b,"getIsGroundCover":b,"getIsTranslucent":b,"getIsHarvestable":b,`
   `"getMaterialMobility":i}` per `Material` static, **sorted by name** (Java
   `String` order); `fields` as for blocks, from `Material` down.
4. 16 lines `{"t":0,"e":"map_color","index":i,"present":b,"fields":{"colorIndex":…,"colorValue":…}}`
   for `MapColor.mapColorArray` (slots 14, 15 are null: `"present":false`, no `fields`).

Reduction rules for field values and method results: `int`/`short`/`byte`/`long`
→ JSON number; `boolean` → JSON boolean; `float`/`double` → bit-pattern strings
(`floatToRawIntBits`); `String` → string; `null` → null; a `Block` → its `blockID`;
a `Material` → its static field name (`"rock"`); a `MapColor` → its `colorIndex`; a
`StepSound` → `{"class","name","volume","pitch","stepSoundDir","stepSoundDir2"}`
(`name` is `field_1678_a`, the others are the getters); an enum → `name()`; a
`Class` → simple name (`BlockSign.signEntityClass`: `"TileEntitySign"`); a
`java.util.Random` → the string `"java.util.Random"` (the unseeded `new Random()`
field initialisers of `BlockChest`, `BlockDispenser`, `BlockFurnace`; state not
dumped); arrays and collections → lists of reduced elements in index/iteration order
(`BlockRedstoneWire.blocksNeedingUpdate` is empty). Any other type is an oracle
error, so a new field cannot slip through unreduced.

Rust side: `crates/mc-sim/tests/golden_block_table.rs` rebuilds every line from
`mc_sim::block` (the table, the data-only functions, `BlockStatics::default()`)
and compares the whole trace with `mc_golden::compare`.

### `kind: "item_table"` — `Item.itemsList`, tool materials, item icons, `ItemStack` (WP-1.2)

```json
{"name":"item-table","tier":"A","kind":"item_table",
 "stacks":[{"id":256,"count":1,"damage":0,"damage_by":[1,248,1,1],"split":1}, ...]}
```
No seed: the output is the vanilla static state after `Block`'s and `Item`'s static
initialisers (the oracle initialises `Block` first, as the game does, then reseeds
`Item.itemRand` with `mix(0, -1)`; nothing draws from it). The kind-specific field
`stacks` is read by the Rust test itself (`crates/mc-sim/tests/golden_item_table.rs`);
`mc-golden` models the kind as a unit variant. Trace, in this order:

1. **items**: one line per id `0..=N`, `N` the highest non-null index of
   `Item.itemsList` (2257). A null entry is `{"t":0,"e":"item","id":N,"present":false}`.
   Otherwise `"present":true`, `"class"` (simple name), and:
   - `"fields"`: an object keyed by every class from the concrete class up to `Item`
     (simple names), each holding that class's declared non-static, non-synthetic
     fields by Java name (empty `{}` for classes with none). Reduction rules: `int`,
     `short`, `byte`, `long` → numbers; `boolean` → booleans; `float`/`double` →
     bit-pattern strings; `String` → string; null → null; `Block` → its `blockID`;
     `Item` → its `shiftedIndex`; `ItemStack` → `{"id","count","damage"}`; `Material`
     → the name of the `Material` static field holding it (`"wood"`); enums →
     `name()`; arrays → lists of reduced elements. Any other type fails the oracle.
   - `"methods"`: the data-method results. Per damage `d` in `0..16` (lists of 16):
     `getIconFromDamage(d)`, `getItemNameIS(new ItemStack(item, 1, d))` (string or
     null), `getPlacedBlockMetadata(d)`, `getColorFromDamage(d)`. Scalars:
     `getItemName` (string or null), `getItemStackLimit`, `getMaxDamage`,
     `isDamagable`, `isFull3D`, `shouldRotateAroundWhenRendering`, `getHasSubtypes`,
     `getContainerItem` (its `shiftedIndex` or null), `hasContainerItem`,
     `getDamageVsEntity(null)`. Per block id `0..256` (lists of 256, null where
     `Block.blocksList[id]` is null): `getStrVsBlock(new ItemStack(item, 1, 0), block)`
     (f32 bit patterns) and `canHarvestBlock(block)`.
2. **tool materials**: one `{"e":"tool_material","name","ordinal","fields":{...},
   "getMaxUses","getEfficiencyOnProperMaterial","getDamageVsEntity","getHarvestLevel"}`
   per `EnumToolMaterial` constant in ordinal order (`fields` as above, for the
   enum class's own fields).
3. **block textures**: one `{"e":"block_texture","id",
   "getBlockTextureFromSide":[6],"getBlockTextureFromSideAndMetadata":[6][16]}` per
   non-null block, sides `0..6`, metadata `0..16` (the lookups the `ItemBlock` icons
   come from; static-init state of the mutable singleton fields).
4. **stacks**: one `{"e":"item_stack","i"}` line per `stacks` entry, in order, for
   `st = new ItemStack(id, count, damage)`: `nbt_types` (tag key → NBT type id of
   `st.writeToNBT(new NBTTagCompound())`), `nbt` (`getShort("id")`,
   `getByte("Count")`, `getShort("Damage")` of it), `read` (`new ItemStack(nbt)`),
   `copy` (`st.copy()`), `getMaxStackSize`, `isStackable`, `isItemStackDamageable`,
   `isItemDamaged`, `getHasSubtypes`, `getMaxDamage`, `getItemName`, `getIconIndex`,
   `getDamageVsEntity(null)`, `damageItem` (on one copy, `damageItem(n, null)` for each
   `n` of `damage_by` in order, the stack after each), and `splitStack`
   (`[rest, part]` of `copy().splitStack(split)`). Stacks are
   `{"id","count","damage","animationsToGo"}`.

Wherever a call throws, the value is `{"exception":"<simple class name>"}` (e.g. an
`ItemSlab` name for damage 4..15, the piston head texture for direction 6/7); the
Rust side must return the matching `mc_sim::item::JavaException`. Only ids with a
non-null item may appear in `stacks` (a null item is a `NullPointerException` the
port does not model).

Scenario: `item/item-table`.

### `kind: "recipe_table"` — `CraftingManager`'s recipe list (WP-2.5)

```json
{"name":"recipe-table","tier":"A","kind":"recipe_table"}
```
No seed and no fields: the output is `CraftingManager.getInstance().getRecipeList()`
after the static initialisers (`Block` first, as for `item_table`), i.e. the recipes in
registration order (CraftingManager.java:16-80) after `Collections.sort` with
`RecipeSorter` (stable: shaped before shapeless, larger `getRecipeSize()` first).
`mc-golden` models the kind as a unit variant; the Rust side is
`crates/mc-sim/tests/golden_recipe_table.rs`. Stacks are `{"id","count","damage"}` or
`null` (the op-line form, `Dumps.stack`). Trace, in this order:

1. `{"t":0,"e":"recipe_count","count":151}`.
2. One line per recipe, `i` its index in the sorted list:
   - shaped: `{"e":"recipe","i","kind":"shaped","width","height","size","input","output",
     "output_item_id","craft","craft_mirrored","craft_2x2"?}`: `input` is the private
     `recipeItems` (`width × height` entries, row-major, `null` for a blank, damage −1 for
     a `Block` ingredient, which matches any damage), `size` is `getRecipeSize()`,
     `output` `getRecipeOutput()`, `output_item_id` the public `recipeOutputItemID`.
   - shapeless: `{"e":"recipe","i","kind":"shapeless","size","input","output","craft",
     "craft_2x2"?}`: `input` is the private `recipeItems` list in argument order.
   - Probes, each `findMatchingRecipe` (the first matching recipe's
     `getCraftingResult`, or `null`) over a new grid filled with
     `setInventorySlotContents`, every ingredient as `new ItemStack(id, 1, damage)` with
     damage −1 replaced by 0: `craft`, a 3×3 grid with the pattern at the top left (cell
     `(col, row)` is slot `col + 3·row`; shapeless: ingredient `k` in slot `k`);
     `craft_mirrored` (shaped), the same with each row reversed; `craft_2x2`, present only
     for a shaped recipe at most 2×2 or a shapeless one of at most 4 ingredients, the
     pattern in a 2×2 grid (slot `col + 2·row`; shapeless: slot `k`).

Scenario: `recipe/recipe-table`.

### `kind: "stat_table"` — `StatList` and `AchievementList` (WP-2.7)

```json
{"name":"stat-table","tier":"A","kind":"stat_table","format_values":[0,1,50,51,1000,62500,-1]}
```
No seed. The registries after vanilla's static initialisers (`Block` first: Block.java:689
starts `Item`, whose last line starts `StatList`, which registers the achievements; then
Block.java:705), run by the oracle in a fresh class loader whose `StringTranslate` table
is an identity table (`tools/oracle/README.md`, "`stat_table`"). So a stat's `name` is its
translation key, and a formatted name (`stat.mineBlock`, `stat.useItem`, `stat.breakItem`,
`stat.craftItem`) is `key(argument key)`, the argument being the block's or item's
`getBlockName()` / `getItemName()` + `".name"`: `"stat.mineBlock(tile.stone.name)"`.
`mc-golden` models the kind as a unit variant; `format_values` (optional, ints) is read
by the Rust side, `crates/mc-sim/tests/golden_stat_table.rs`. Trace, in this order:

1. `{"e":"stat_count","listed","oneShot","generalStats","itemStats","objectMineStats",
   "achievements"}`: the sizes of `StatList.field_25188_a`, `oneShotStats`,
   `generalStats`, `itemStats`, `objectMineStats` and `AchievementList.achievementList`.
2. One `{"e":"stat","i","statId","name","class","type","isIndependent","itemId"?}` per
   entry of `field_25188_a`, `i` its index: `class` is the stat's class simple name
   (`StatBasic`, `StatCrafting`, `Achievement`), `type` its `IStatType`'s
   (`StatTypeSimple`, `StatTypeTime`, `StatTypeDistance`), `itemId` the
   `StatCrafting`'s block or item id.
3. One `{"e":"unlisted_stat",...}` (the same fields, no `i`) per stat of `oneShotStats`
   missing from `field_25188_a` (removed by `replaceSimilarBlocks`), by `statId`.
4. `{"e":"stat_list","list","ids"}` for `generalStats`, `itemStats`, `objectMineStats`
   in that order: the `statId`s in list order.
5. `{"e":"stat_array","array","length","entries"}` for `mineBlockStatArray`,
   `objectUseStats`, `objectBreakStats`, `objectCraftStats` in that order: `entries`
   are the non-null slots as `[index, statId]`, by index (aliases from
   `replaceSimilarBlocks` show as two indices with one `statId`).
6. One `{"e":"achievement","i","statId","name","column","row","parent","icon","isSpecial",
   "isIndependent"}` per `achievementList` entry: `parent` the parent's `statId` or
   `null`, `icon` `{"id","damage"}` of `theItemStack`.
7. `{"e":"achievement_bounds","minDisplayColumn","minDisplayRow","maxDisplayColumn",
   "maxDisplayRow"}`.
8. For `StatTypeSimple`, `StatTypeTime`, `StatTypeDistance` in that order, one
   `{"e":"stat_format","type","v","s"}` per `format_values` entry: `s` is
   `format(v)`, with `StatBase`'s `DecimalFormat` created under `Locale.US`.

Scenario: `stat/stat-table`.

## Tier B/C/S scenarios (WP-0.4)

Common fields: `"name"` (the directory name), `"tier"` ∈ `B` | `C` | `S`, `"kind"`
∈ `tick` (any of the three tiers) | `world_read` (S only). `"kind"` must be the first
`"kind"` key in the file (ops also have one; `tools/oracle/check-sensitivity.sh` reads
the first). Unknown extra keys such as `"comment"` are ignored. The oracle side
(seeding protocol, vanilla call chains) is in `tools/oracle/README.md`; the
nondeterminism census behind it in `tools/oracle/census.md`.

### `kind: "tick"` — a vanilla world ticked under a script

```json
{"name":"flat-item-drop","tier":"C","kind":"tick","seed":123,"dimension":"overworld",
 "world":"empty","save_format":"none","difficulty":0,"ticks":60,
 "ops":[{"t":0,"op":"fill","x0":-8,"y0":60,"z0":-8,"x1":8,"y1":60,"z1":8,"id":1},
        {"t":0,"op":"spawn","kind":"Item","item":{"id":4,"count":1,"damage":0},
         "pos":["0x3fe0000000000000",70,"0x3fe0000000000000"],"id":0},
        {"t":0,"op":"set_motion","id":0,"motion":["0x3fb999999999999a","0x3fc999999999999a","0x3fa999999999999a"]}],
 "dumps":[{"every":1,"what":"entity_state","which":"all"}]}
```

| Field | Meaning |
|---|---|
| `seed` | required; the world seed and the root of every harness seed |
| `dimension` | `overworld` (WorldProviderSurface) \| `nether` (WorldProviderHell) |
| `ticks` | number of ticks, ≥ 1; ticks are numbered 0 … ticks−1 |
| `world` | `generated` (default; the dimension's terrain generator) \| `empty` (every chunk all air) \| `load` (a saved world, `dir`; see "Loaded worlds" below) |
| `dir` | `world: "load"` only: the save directory (`level.dat`, `region/`), relative to the scenario directory; `..` may climb up to the fixture tree `tests/golden` (so `"../../save/testworld/world"` reuses the committed testWorld) |
| `save_format` | `none` (default; `SaveHandlerMP`, nothing on disk) \| `mcregion` (`SaveOldDir` on a fresh temp directory; needed by `save`) |
| `mob_spawning` | `default` (default) \| `none`. A Tier B isolation knob: `none` makes the tick loop call `setAllowedMobSpawns(false, false)` instead of `(difficulty > 0, true)` (step 6 below), so `SpawnerAnimals.performSpawning` returns before drawing anything (SpawnerAnimals.java:20-21). For scenarios about blocks that need a player for random ticks but must not depend on the spawner |
| `difficulty` | 0–3, default **0**; `world.difficultySetting` every tick and `setAllowedMobSpawns(difficulty > 0, true)` (Minecraft.java:1142, :1163). Not GameSettings' default 2: hostile spawn checks draw from an unseeded `Entity.rand` (census §3), so hostile spawning is not reproducible; the oracle warns when a scenario with a player sets it above 0 |
| `ops` | list of `{"t": tick, "op": name, …}`; applied in file order within a tick |
| `dumps` | list of `{"what": name, "every": N}` (due when `(t + 1) % N == 0`, i.e. after every N ticks) or `{"what": name, "t": T}`; due dumps run in list order |

Numbers: block coordinates, ids, metas and counts are JSON integers. Entity positions,
motions and angles (Java `double`/`float`) are a JSON integer (exact) or a bit-pattern
string (`f64` 16 hex digits, `f32` 8), as in the Encoding rules.

**World.** `new World(handler, "harness", provider, seed)` (World.java:62-101): a fresh
`WorldInfo(seed, "harness")` with spawn (0, 0, 0), no chunk loaded, no spawn-point search;
then the Tier A overrides (`rand.setSeed(seed)`, `field_9437_g` and `soundCounter` from
`new Random(seed)`, see `terrain`). `"world": "empty"` replaces the provider's terrain
generator inside the vanilla `ChunkProvider` with one whose chunks are
`new Chunk(world, new byte[32768], x, z)` + `generateSkylightMap()` (Chunk.java:48-54,
:88-131): height map 0, sky light 15 at y = 127…1 and 0 at y = 0, block light 0;
`populate` does nothing (the vanilla wrapper still marks the chunk populated and
modified). Before the first tick the harness reseeds the `Math.random()` stream with
`seed` and resets the static state listed in census §6.

**Seeding.** `mix(s, k) = s ^ (k * 0x9E3779B97F4A7C15)` (wrapping `i64`). Every time an
entity joins the world (`World.entityJoinedWorld` or a chunk load's
`addLoadedEntities`) its `rand` is set to `mix(seed, entityId)`; equivalently for a
port: an entity's `rand` is seeded with `mix(seed, id)` when it is constructed and
again whenever it joins. Entity ids count from 0 per scenario in construction order.
Singletons: `Item.itemRand` = `mix(seed, -1)`, `Block.chest`'s = `mix(seed, -2)`,
`Block.stoneOvenIdle`'s = `mix(seed, -3)`, `Block.stoneOvenActive`'s = `mix(seed, -4)`,
`Block.dispenser`'s = `mix(seed, -5)`; an `explode` op's `ExplosionRNG` = `mix(seed, -6)`
(the port seeds every explosion's so; a `TileEntityDispenser`'s own `Random` has no hook
and is `mix(seed, -7)` in the port only, census §2). The `Math.random()` stream is
`JavaRandom::new(seed)`, drawn only at census §1's S and D sites.

**Tick loop.** Before tick 0 the implicit preload runs (below) and emits
`{"t":0,"e":"preload","implicit":true,"loaded":N}`. Then for each `t`:

1. `AxisAlignedBB.clearBoundingBoxPool()`, `Vec3D.initialize()` (frame start, Minecraft.java:519-520).
2. The ops with this `t`, in file order.
3. If there is a player: `if (++joinPlayerCounter == 30) { joinPlayerCounter = 0; world.joinEntityInSurroundings(player); }` (Minecraft.java:1134-1139).
4. `world.difficultySetting = difficulty`; `if (world.field_27172_i > 0) --world.field_27172_i` (Minecraft.java:1142, :1155-1157).
5. `world.updateEntities()` (World.java:1178-1276).
6. `world.setAllowedMobSpawns(difficulty > 0, true)` (`(false, false)` with `"mob_spawning": "none"`); `world.tick()` (World.java:1745-1780).
7. If there is a player: `world.randomDisplayUpdates(floor(posX), floor(posY), floor(posZ))` (Minecraft.java:1167-1169; 6000 `world.rand` draws, World.java:1997-2011).
8. `world.updatingLighting()` once (Minecraft.java:553: once per frame, at most 500 updates; the harness runs one frame per tick).
9. The dumps due at `t`.

So a tick's lines are its op lines, then its dump lines; a dump at `t` sees the state
after tick `t` (world time `t + 1` when not set). The real client may run several
frames per tick or several ticks per frame; the one-frame-per-tick model fixes the
lighting and pool cadence for both engines.

**Preload.** A preload of chunk square (cx, cz, r) calls `getChunkFromChunkCoords` for
`dz` in −r…r (outer), `dx` in −r…r (inner), and after each chunk drains the light queue
(`while (world.updatingLighting());`, as Minecraft's world start does,
Minecraft.java:1366-1373). The implicit preload: for every position named by a
`set_block`, `set_block_silent`, `fill` (both corners), `player`, `spawn`, `set_pos` or
`use_item` op, in op file order, the square r = 2 around chunk `(floor(x) >> 4, floor(z) >> 4)`,
i.e. every chunk within 32 blocks: an entity only ticks when all chunks within 32 blocks
exist (World.java:1291-1296), and `entityJoinedWorld` refuses a non-player entity whose
chunk is not loaded (World.java:904-906). `loaded` counts the chunks of the requests that
were not loaded yet (population and moving entities may load more).

**Ops.** Every op emits one line `{"t":t,"e":"op","op":name, …}` with the fields below.

| Op | Fields | Vanilla path | Line fields |
|---|---|---|---|
| `set_block` | `x,y,z,id,meta?` (meta 0) | `World.setBlockAndMetadataWithNotify` (World.java:461-468): `Chunk.setBlockIDWithMetadata` (old block's `onBlockRemoval`, new block's `onBlockAdded`, light updates queued, Chunk.java:236-273) then `notifyBlockChange` (neighbours' `onNeighborBlockChange`, never the block itself). `onBlockAdded` is where e.g. flowing water and sand schedule their first tick | `ok` (the return value) |
| `set_block_silent` | same | `World.setBlockAndMetadata` (World.java:370-384): as above without `notifyBlockChange`; `onBlockAdded` still runs | `ok` |
| `fill` | `x0,y0,z0,x1,y1,z1,id,meta?` | `set_block_silent` over the box, x outer, z middle, y inner | `changed` (count of `true` returns) |
| `set_time` | `time` | `World.setWorldTime` (World.java:2274) | — |
| `set_weather` | `rain_time?`, `raining?`, `thunder_time?`, `thundering?` (int, bool, int, bool) | the `WorldInfo` setters `setRainTime`, `setRaining`, `setThunderTime`, `setThundering` (WorldInfo.java:195-221) for the fields given. A Tier B knob to reach `updateWeather`'s toggles (World.java:1792-1859) without waiting 12000+ ticks | — |
| `preload` | `cx,cz,radius` | preload square (above) | `loaded` |
| `player` | `x,y,z` (feet), `yaw?,pitch?`, `inventory?` (list of `{slot,id,count,damage?}`) | `new HarnessPlayer(world)` (username `"Player"`, as an `EntityPlayerSP`'s session gives it), seeded, each `inventory` entry `inventory.setInventorySlotContents(slot, new ItemStack(id, count, damage))` (InventoryPlayer.java:180-188: slot 0–35 main, 0–8 the hotbar, 36–39 armour, `armorInventory[slot - 36]`; the armour slot of a piece of `armorType` t is `39 - t`, as `ContainerPlayer` places it), `setLocationAndAngles`, `entityJoinedWorld`. At most one | `id`, `joined` |
| `spawn` | `kind`, `pos:[x,y,z]` (bounding-box bottom), `yaw?,pitch?`, `id?`; for `kind: "Item"`: `item:{id,count,damage?}`, `pickup_delay?` (10); for `kind: "Painting"`: `direction?` (0–3); for `kind: "Minecart"`: `type?` (0 plain, 1 chest, 2 furnace) | `new Entity<kind>(world)` with the class from EntityList's name table (EntityList.java:82-107: `Arrow`, `Snowball`, `Item`, `Painting`, `Mob`, `Monster`, `Creeper`, `Skeleton`, `Spider`, `Giant`, `Zombie`, `Slime`, `Ghast`, `PigZombie`, `Pig`, `Sheep`, `Cow`, `Chicken`, `Squid`, `Wolf`, `PrimedTnt`, `FallingSand`, `Minecart`, `Boat`); `rand.setSeed(mix(seed, id))` (Slime: size and jump delay redrawn); `setLocationAndAngles(x, y, z, yaw, pitch)` (Entity.java:669-676: `posY = y + yOffset`; yaw/pitch default to the constructor's, a `Math.random()` yaw for living entities); `entityJoinedWorld`. `Item` instead uses `new EntityItem(world, x, y, z, stack)` (EntityItem.java:11-21: `Math.random()` yaw and horizontal motion, `motionY` 0.2, position via `setPosition`) and `delayBeforeCanPickup = pickup_delay`, as block drops do (Block.java:355-364). Constructors that take the position (no `setLocationAndAngles` after them) and the harness-defined kinds (below). `id`, if given, must equal the entity id it gets | `id`, `kind` (class simple name), `joined` |
| `set_motion` | `id`, `motion:[x,y,z]` | assigns `motionX/Y/Z` | `id` |
| `set_pos` | `id`, `pos:[x,y,z]` | `Entity.setPosition` (Entity.java:163-170; `prevPos*` untouched) | `id` |
| `set_rotation` | `id`, `yaw`, `pitch` | assigns `rotationYaw`/`rotationPitch` (no `% 360`) | `id` |
| `use_item` | `item_id?`, `count?` (1), `meta?` (0), `x,y,z,side` | needs a player. The stack (if `item_id` is given) replaces hotbar slot `currentItem`; then the right-click-on-block branch of `Minecraft.clickMouse(1)` (Minecraft.java:819-850) with the op's hit instead of a ray trace: `PlayerController.sendPlaceBlock` (PlayerController.java:75-82: the block's `blockActivated`, else `ItemStack.useItem` → `Item.onItemUse`); on success `swingItem`; a stack left at size 0 is removed; if nothing was placed, `sendUseItem` (`useItemRightClick`, PlayerController.java:47-61). The op's block only feeds `sendPlaceBlock`: the items that aim (`ItemBucket`, `ItemBoat`) trace their own eye ray from the player's position and angles (`set_rotation` turns it) | `placed`, `used`, `stack` (`{id,count,damage}` or `null`, the slot afterwards) |
| `interact` | `id`, `item_id?`, `count?` (1), `meta?` (0) | needs a player. The stack (if `item_id` is given) replaces hotbar slot `currentItem`; then the right-click-on-entity branch of `Minecraft.clickMouse(1)` (Minecraft.java:800-817, :846-850) with the op's entity instead of the ray trace: `PlayerController.interactWithEntity` (PlayerController.java:88-90) = `EntityPlayer.useCurrentItemOnEntity` (EntityPlayer.java:457-469), `result = entity.interact(player)` (a boat or an empty minecart takes the player on, or puts it off when it already rides it; a furnace minecart takes a coal; a chest minecart opens its GUI), and when that is false the held stack's `useItemOnEntity` on a living entity (ItemStack.java:170-172) and its removal at size ≤ 0; then, as `clickMouse` leaves `var2` true on this branch, `sendUseItem` with the held stack (PlayerController.java:47-61) | `id`, `result`, `used`, `stack` (`{id,count,damage}` or `null`, the slot afterwards) |
| `input` | `forward?`, `strafe?` (f32, default 0: the key sums, +1 forward / left), `jump?`, `sneak?` (default false), `hotbar?` (0–8) | needs a player. From now until the next `input` op the player's `MovementInput` is what `MovementInputFromOptions.updatePlayerMoveState` makes of those keys (MovementInputFromOptions.java:50-76): `moveForward = forward`, `moveStrafe = strafe`, both `(float)((double)v * 0.3D)` while `sneak`. `HarnessPlayer` applies it where `EntityPlayerSP` does: after `EntityPlayer.updatePlayerActionState`, `moveStrafing`/`moveForward`/`isJumping` from it (EntityPlayerSP.java:27-32), and `isSneaking() = sneak && !sleeping` (:144-146; `Entity.moveEntity`'s edge guard and the ladder hold read it). Not the `ySize` dip of sneaking (EntityPlayerSP.java:79-81, client part of `onLivingUpdate`). `hotbar` is a number key: `inventory.currentItem = hotbar` (Minecraft.java:1073-1076) | — |
| `dig` | `x,y,z,side` | needs a player; at most one per tick. The left button held on that block face this tick, with the op's block instead of the ray trace (`Minecraft.runTick`'s mouse handling with `PlayerControllerSP`, Minecraft.java:779-830, :962-964, :1003-1022, :1110-1113): `updateController` (`prevBlockDamage`); then on a press (no `dig` last tick, or another block) or when 5 ticks (`ticksPerSecond / 4`) passed since the last click, `clickMouse(0)`: `swingItem`, `clickBlock` (`World.onBlockHit` puts out fire in front of the face; the block's `onBlockClicked` when `curBlockDamage == 0`; `sendBlockRemoved` at once when `blockStrength(player) >= 1`); then `sendBlockRemoving` (after a break `blockHitWait` 5 ticks; the same block as last time adds `blockStrength` and breaks at 1; another block resets the damage to 0 that tick). `sendBlockRemoved`: aux effect 2001, the block set to air, `onBlockDestroyedByPlayer`, the held stack's `onDestroyBlock` (tool wear), `harvestBlock` (the drops) when the player could harvest it (PlayerControllerSP.java:126-199, PlayerController.java:21-32). A tick without a `dig` op (after one) releases the button: `updateController`, `resetBlockRemoving` | `x`, `y`, `z`, `clicked` (whether `clickMouse(0)` ran), `progress` (`curBlockDamage` afterwards, f32 bits), `broken` (a `sendBlockRemoved` of this tick returned true) |
| `sleep` | `x,y,z` | needs a player. `Block.blockActivated(world, x, y, z, player)` on the block there (a bed: BlockBed.java:14-83, which calls `EntityPlayer.sleepInBedAt`, EntityPlayer.java:537-596) | `result` (`blockActivated`'s value), `status`: the name of the `EnumStatus` `sleepInBedAt` returned (`OK`, `NOT_POSSIBLE_HERE`, `NOT_POSSIBLE_NOW`, `TOO_FAR_AWAY`, `OTHER_PROBLEM`; b1.7.3 has no `NOT_SAFE`), or `null` when the bed did not call it (another player sleeps there; no bed half with bit 3; not the overworld, where the bed explodes) |
| `attack` | `target_id`, `attacker_id?`, `damage` (unless the attacker is the player) | attacker = the player: `swingItem` then `attackTargetEntityWithCurrentItem` (Minecraft.java:802-814, PlayerController.java:92-94). Otherwise `target.attackEntityFrom(attacker or null, damage)` (Entity.java:746, EntityLiving.java:307) | `target_id`, `via` (`player` \| `direct`), `result` (direct only) |
| `explode` | `x,y,z` (f64: integer or bit pattern), `size` (f32), `flaming?` (false) | `World.createExplosion(null, x, y, z, size)` (World.java:1518-1520: `newExplosion(…, false)`, Explosion.java:29-164). With `flaming`: the body of `World.newExplosion(null, x, y, z, size, true)` (World.java:1522-1528) with the explosion's `ExplosionRNG` (a clock-seeded `new Random()`, Explosion.java:11, census §2) set to `mix(seed, -6)` before `doExplosionA`; only the fire roll draws it (Explosion.java:118). No exploder: no entity is excluded, the attacker is null. Not part of the implicit preload (WP-1.8) | `destroyed` (size of `destroyedBlockPositions`) |
| `craft` | `grid`: 9 (3×3) or 4 (2×2) entries, each `null` or `[id, count, damage]` | `CraftingManager.getInstance().findMatchingRecipe(grid)` (CraftingManager.java:160-169) over a new `InventoryCrafting` of that size, entry `i` put into slot `i` (column `i % w`, row `i / w`) with `setInventorySlotContents`; the grid's event handler is a `Container` without slots, so nothing else runs. Needs no player (WP-2.5) | `result` (`{id,count,damage}` or `null`: the first matching recipe's `getCraftingResult`) |
| `slot_click` | `container`, `slot`, `button`, `shift?` (false) | needs a player. `container.slotClick(slot, button, shift, player)` (Container.java:47-150), as `PlayerController.handleMouseClick` passes a GUI click (PlayerController.java:96-98), with the player's `inventory.getItemStack()` as the stack on the cursor. `container`: `"player"` is the player's own `inventorySlots` (the `ContainerPlayer` its constructor built, EntityPlayer.java:42); any other spec is opened by the op: `"workbench"` (`new ContainerWorkbench(inventory, world, 0, 0, 0)`; the position only feeds `isUsableByPlayer`, which `slotClick` never calls), `"workbench at x,y,z"`, `"chest at x,y,z"` (`new ContainerChest(inventory, chest)` on the inventory `BlockChest.blockActivated` would open, BlockChest.java:186, :198-212: the chest's tile entity wrapped in `InventoryLargeChest("Large chest", …)` with each neighbouring chest, −x and −z as the upper half, +x and +z as the lower; the normal-cube checks that may refuse to open are not applied), `"furnace at x,y,z"` (`ContainerFurnace` on the `TileEntityFurnace` there), `"dispenser at x,y,z"` (`ContainerDispenser`). A spec equal to the previous non-`"player"` one reuses that container (its crafting grid survives); a different spec replaces it without `onCraftGuiClosed` (a workbench grid's stacks are then lost). `slot` is a slot number of the container (each container class's module documents its layout, e.g. player: 0 output, 1–4 grid, 5–8 armour helmet first, 9–35 main, 36–44 hotbar) or −999 (outside the window: the held stack, or one item of it, goes to `dropPlayerItem`); any other value is a scenario error. `button` 0 left, 1 right (others do nothing). `shift` runs the container's `getStackInSlot(slot)` override (b1.7.3's shift-click move) | `slot`, `result` (`slotClick`'s return value: a copy of the slot's stack before the click, or `null`), `stack` (`getSlot(slot).getStack()` afterwards; `null` for −999), `held` (`inventory.getItemStack()` afterwards) |
| `save` | `chunks?` (list of `{x,z}`) | Tier S, below; needs `"save_format": "mcregion"` or `"world": "load"` | — then the Tier S lines |
| `reload` | — | quit the world and open its save again in the same run; needs `"save_format": "mcregion"` or `"world": "load"`; see "Reload" below (WP-2.G) | `chunks` (loaded chunks afterwards), `entities` (`loadedEntityList` size), `tile_entities` (`loadedTileEntityList` size), `player` (the new player's entity id, or `null` without a `player` op) |

**Loaded worlds** (`"world": "load"`, WP-2.5a). The save directory `dir` is copied to a
temp directory and opened with the loading constructor `World(SaveOldDir, "harness", seed,
null)` (World.java:149-208), as `world_read` opens one: the dimension and the world seed come
from `level.dat` (`dimension` is required but ignored), `seed` only roots the harness streams
(the determinism overrides and `resetStatics` run with it). No `save_format`: the copy is the
world's save, so autosaves (every 40 ticks) and `save` ops write there, and a chunk missing
from it is generated and populated as vanilla does. The `level.dat` player is not applied
(`spawnPlayerWithLoadedChunks` is the client's); a `player` op adds the harness player. The
Rust side needs the fixture directory: `mc_golden::tick_driver::drive_in`.

**Reload** (`reload` op, WP-2.G; oracle `HarnessWorld.reload`, Rust
`tick_driver::reload`). What a player does by quitting to the title screen and loading the
world again, in one run, so a scenario can check that everything that matters survives a
save: in this order

1. the old world's loaded chunks are listed in `ChunkProvider.chunkList` (load) order;
2. `World.saveWorld(true, null)` (World.java:278-291), as `Minecraft.changeWorld` saves the
   world it leaves (`saveWorldIndirectly`, Minecraft.java:1276, World.java:1653-1655); the old
   `World` is then dropped;
3. the save is opened with the loading constructor `World(SaveOldDir(<save root>, "world"),
   "harness", seed, WorldProvider.getProviderForDimension(old.worldProvider.worldType))`
   (World.java:149-208, WorldProvider.java:100-108): seed, time, weather and the `Player`
   compound come from `level.dat` as for `"world": "load"`, but the dimension is the old
   world's. This is a harness choice: with a null provider the dimension would come from the
   `Player` compound's `Dimension` (World.java:186-190, WorldInfo.java:38-41), which is 0 for
   a world without a player (no `Player` tag) and for the harness player (its
   `Entity.dimension` is never set), so a nether world would come back as a surface world
   reading `world/region` instead of `world/DIM-1/region` (SaveOldDir.java:11-20);
   `tick/reload-nether` pins this down. `"world": "empty"` gets its empty generator again;
4. **what is carried over and why.** The constructor draws `field_9437_g` and `rand` from
   clock-seeded `new Random()`s and `soundCounter` from that `rand` (World.java:162, :169,
   :177): they have no vanilla-reproducible value, so the harness defines them as the old
   world's: the `World.rand` object itself (its 48-bit state and a pending
   `nextNextGaussian`), `field_9437_g` and `soundCounter`. Nothing is reseeded
   (`makeDeterministic`/`resetStatics` do not run): the JVM statics go on as they would in the
   game: `Entity.nextEntityID` (so reloaded and new entities keep counting up),
   `NextTickListEntry.nextTickEntryID`, the `Math.random()` stream, `Item.itemRand`, the
   block singletons' `Random`s and flags, `World.lightingUpdatesScheduled`,
   `SpawnerAnimals.eligibleChunksForSpawning`. The seeding hook is registered on the new
   world (same seed); the `dig` controller, the last dug block and the container of the last
   `slot_click` are dropped (they belong to the old world);
5. the listed chunks are loaded again in that order, each followed by draining the light
   queue (as a preload does): saved ones from the region files with their entities (new ids,
   seeded `mix(seed, id)` by the join hook) and tile entities; chunks that were never saved
   (`Chunk.needsSaving`, Chunk.java:536-550: unmodified and without entities) are generated
   again, and population follows the new load order;
6. if the scenario has a player: a new `HarnessPlayer`, seeded after construction as the
   `player` op does, then `world.spawnPlayerWithLoadedChunks(player)` (World.java:256-276:
   `readFromNBT` of `level.dat`'s `Player` compound, which is then cleared, and
   `entityJoinedWorld`), as `Minecraft.changeWorld` ends (Minecraft.java:1325). The client's
   `preparePlayerToSpawn`/`flipPlayer` (Minecraft.java:1298-1300) are not called, as the
   `player` op does not call them (`readFromNBT` overwrites what they set). The new player has
   a new id (the line's `player`), a new `ContainerPlayer` with an empty crafting grid and no
   stack on the cursor (neither is saved), hotbar selection 0 (`currentItem` is not saved) and
   no movement input until the next `input` op; a sleeping player is woken by `readFromNBT`
   (EntityPlayer.java:315-318).

Not carried over, as in vanilla: **scheduled ticks** (b1.7.3 does not save them:
`ChunkLoader.storeChunkInCompound`, ChunkLoader.java:108-149, writes no tick list, so the
reload drops them on both sides; `tick/reload-roundtrip` shows sand left in the air), the
light queue, rain/thunder strengths (the constructor's `func_27163_E` sets them to 1 when it
rains, World.java:1782-1790), `InventoryPlayer.currentItem`, the cursor stack and the crafting
grid, `EntityItem.delayBeforeCanPickup` and the other fields entity NBT does not hold.
The Rust side: `World::new_from_save` on a new `SaveOldDir` over the same in-memory store,
`World::set_chunk_generator` for an empty world, `World::adopt_clock_seeded_state` (rand,
`field_9437_g`, `soundCounter`) and `World::adopt_statics` (the Java statics the port keeps
per world), `World::loaded_chunk_coords` for the chunk order, `World::spawn_player_with_loaded_chunks`.

**`spawn` kinds beyond `EntityList`'s `(World)` constructors** (WP-2.4). Each is seeded right
after construction like every `spawn`; where the vanilla constructor draws from the then
unseeded `Entity.rand` (census §3), the oracle replays those draws on the seeded stream (as it
does for the slime), so the port's constructor, whose `rand` is seeded at construction, draws
the same values:

| `kind` | Vanilla constructor | Placement, join | Replayed draws |
|---|---|---|---|
| `Painting` with `direction` | `new EntityPainting(world, floor(x), floor(y), floor(z), direction)` (EntityPainting.java:22-45, as ItemPainting.java:27): every motive that fits the wall block `floor(pos)` facing `direction` (0: −z face, 1: −x, 2: +z, 3: +x) is a candidate | no `setLocationAndAngles`; `entityJoinedWorld` | the art choice `rand.nextInt(candidates)` (:27-44 rerun with the public `art`, `setDirection`, `onValidSurface`) |
| `Egg` (harness-defined: EntityList has no egg) | `new EntityEgg(world)` (EntityEgg.java:16-19) | as an EntityList kind | none |
| `Fireball` (harness-defined) | `new EntityFireball(world)` (EntityFireball.java:19-22): no acceleration | as an EntityList kind | none |
| `LightningBolt` (harness-defined) | `new EntityLightningBolt(world, x, y, z)` (EntityLightningBolt.java:10-34) | no `setLocationAndAngles`; `world.addWeatherEffect` (World.java:891-894, as World.java:1927 does): it is in `weatherEffects`, never in `loadedEntityList`, so `entity_state` does not list it (its effects show in blocks, struck entities and `rng_state`). Refused at `difficulty` ≥ 2, where the constructor also places fire with the unseeded `rand` (:16-32) | `boltVertex = rand.nextLong()`, `boltLivingTime = rand.nextInt(3) + 1` (:14-15) |
| `Minecart` with `type` | `new EntityMinecart(world, x, y, z, type)` (EntityMinecart.java:57-67, as ItemMinecart.java:16): `setPosition(x, y + yOffset, z)`, `prevPos` = `(x, y, z)`, still | no `setLocationAndAngles`; `entityJoinedWorld` | none |
| `FishingHook` (harness-defined) | `new EntityFish(world, player)` (EntityFish.java:48-74, as ItemFishingRod.java:26): needs the `player` op; sets `player.fishEntity`; starts at the player's eyes along its look. `pos` is only used by the implicit preload | no `setLocationAndAngles`; `entityJoinedWorld` | `calculateVelocity`'s three `nextGaussian` (:69-73 rerun from the player's yaw and pitch) |

A bobber dies on its first update unless the player holds a fishing rod (EntityFish.java:145-150):
give it one with `use_item` on a workbench (`BlockWorkbench.blockActivated` returns true, so the
rod is not used).

**Dumps.** A dump never loads a chunk; reading an unloaded one is a scenario error.

| `what` | Fields | Line |
|---|---|---|
| `chunk_digest` | `x,z` (chunk), `full?` | the Tier A `chunk_digest` line (+ `chunk_full`) with `"stage":"tick"` |
| `block` | `x,y,z` | `{"e":"block","x","y","z","id","meta","skylight","blocklight"}`: `getBlockId`, `getBlockMetadata`, `getSavedLightValue(Sky / Block)` (World.java:311, :405, :646-667) |
| `block_box` | `x0,y0,z0,x1,y1,z1` (inclusive) | `{"e":"blocks_full","x0",…,"z1","ids","metas","skylight","blocklight"}`, each a hex string of one byte per block, x outer, z middle, y inner (y fastest, as `Chunk.blocks`) |
| `scheduled_ticks` | `full?` | `{"e":"scheduled_ticks","count","blake3"}` over `World.scheduledTickTreeSet` in TreeSet order (`NextTickListEntry.comparer`: `scheduledTime`, then creation order, NextTickListEntry.java:37-47), 24 bytes per entry: `xCoord`, `yCoord`, `zCoord`, `blockID` as i32 LE, `scheduledTime` as i64 LE. With `full`, also `"entries":[[x,y,z,blockID,scheduledTime], …]`. Quirk: an entry scheduled with blockID 0 keeps scheduledTime 0 (World.java:1165-1167) |
| `entity_state` | `which`: `"all"` \| `[id, …]` | one `entity` line per entity in `loadedEntityList` order (filtered to `which`), then `{"e":"entity_absent","id"}` for each requested id not in the list |
| `world_time` | — | `{"e":"world_time","time"}` (`getWorldTime`) |
| `rng_state` | — | `{"e":"rng_state","world_rand","lcg","math_random"}`: the 48-bit state of `World.rand`, `field_9437_g`, the 48-bit state of the `Math.random()` stream — the fastest first-divergence signal |
| `ray_trace` | `from:[x,y,z]`, `to:[x,y,z]` (f64: integer or bit pattern), `stop_on_liquid?` (false) | `{"e":"ray_trace","from","to","stop_on_liquid","hit"}`: `World.rayTraceBlocks_do(Vec3D.createVector(from), Vec3D.createVector(to), stop_on_liquid)` (World.java:708-861). `from`/`to` echo the inputs as f64 bit strings. `hit` is `null` or the `MovingObjectPosition`: `{"type":"TILE","x","y","z","side","hit_vec":[3 f64 bit strings],"entity":null}` (`blockX/Y/Z`, `sideHit`, `hitVec`, `entityHit`; MovingObjectPosition.java:12-19). Every chunk of the block box spanned by `from` and `to` (x and z) must be loaded |
| `power` | `x,y,z` | `{"e":"power","x","y","z","powered","indirectly_powered"}`: `World.isBlockGettingPowered`, then `World.isBlockIndirectlyGettingPowered` (World.java:2151-2190). They read blocks up to two away horizontally; those chunks must be loaded |
| `inventory` | — | needs a player. `{"e":"inventory","current_item","main","armor","held","craft_matrix","craft_result"}`: `InventoryPlayer.currentItem`, `mainInventory[0..35]` and `armorInventory[0..3]` as lists of `{id,count,damage}` or `null`, the cursor stack `getItemStack()`, and the player's `ContainerPlayer` 2×2 `craftMatrix` (slots 0–3) and `craftResult` (WP-2.5) |
| `map_data` | `id` | `{"e":"map_data","id","present"}`, and when present `"colors","scale","xCenter","zCenter","dimension","step"`: the `MapData` `"map_<id>"` as the world's `MapStorage` (`World.field_28108_z`) holds it in `loadedDataMap` (no file is read, nothing is added): BLAKE3 over `colors` (MapData.java:13; 16384 bytes for a map made by the game), `scale`, `xCenter`, `zCenter`, `dimension` (bytes and ints as numbers) and `step` (MapData.java:9-14). WP-2.6 |
| `player_state` | — | needs a player. `{"e":"player_state","id","sleeping","sleepTimer","bed","spawn","isSwinging","swingProgressInt","swingProgress","width","height","yOffset","ySize","cameraYaw","score","damageRemainder","timeUntilPortal","inPortal","sneaking"}`: the `EntityPlayer` state an `entity` line lacks (EntityPlayer.java:7-38): `isPlayerSleeping()`, `getSleepTimer()`, `bedChunkCoordinates` and `getPlayerSpawnCoordinate()` as `[x,y,z]` or `null`, the arm swing (`swingProgress` f32 bits), the box size, `yOffset`, `ySize`, `cameraYaw` (f32 bits), `score`, the private `damageRemainder` and `inPortal`, `timeUntilPortal`, `isSneaking()` |
| `weather` | — | `{"e":"weather","rain_time","raining","thunder_time","thundering","prev_rain_strength","rain_strength","prev_thunder_strength","thunder_strength"}`: the `WorldInfo` timers and flags (WorldInfo.java:191-221) and `World`'s `prevRainingStrength`, `rainingStrength`, `prevThunderingStrength`, `thunderingStrength` (World.java:27-30; f32 bits) |

`entity` line: `idx` (list index), `id` (`entityId`), `kind` (Java class simple name, e.g.
`EntityPig`, `HarnessPlayer`), `posX/Y/Z`, `prevPosX/Y/Z`, `motionX/Y/Z` (f64 bits),
`rotationYaw`, `rotationPitch` (f32 bits), `onGround`, `isCollidedHorizontally`,
`isCollidedVertically`, `fallDistance` (f32 bits), `isDead`, `ticksExisted`, `rand`
(48-bit state of `Entity.rand`), and `state`, an object with the kind-specific fields:

- `EntityLiving` and subclasses: `health`, `attackTime`, `hurtTime`, `deathTime`,
  `moveForward`, `moveStrafing` (f32 bits), `isJumping`.
- `EntityPlayer` additionally: `currentItem`, `inventory` = BLAKE3 over 40 × 12 bytes,
  `mainInventory[0..35]` then `armorInventory[0..3]`, each `itemID`, `stackSize`,
  `itemDamage` as i32 LE, an empty slot as (−1, 0, 0).
- `EntityItem`: `item` (`{id,count,damage}`), `age`, `delayBeforeCanPickup`, `health`.
- `EntityArrow`: `inGround`, `ticksInAir`, `arrowShake`.

### Tier S: the `save` op and `kind: "world_read"`

Saved files are compared as parsed trees, never as bytes (PLAN_v4 §4.3). The `save` op
calls `World.saveWorld(true, null)` (World.java:277-290: session-lock check, `level.dat`
through `SaveOldDir.saveWorldInfoAndPlayer`, which sets `version` 19132,
SaveOldDir.java:22-25; then every chunk with `needsSaving(true)`, Chunk.java:536-550,
through `McRegionChunkLoader.saveChunk`). Note `tick()` also autosaves when the new
world time is a multiple of 40 (World.java:1772-1775), which clears chunks' modified
flags. Then, from the save directory, in this order:

1. `{"e":"level_dat","root":<canonical tree>,"masked":[…]}`: `level.dat` read with the
   vanilla gzip reader; canonical tree as in `nbt_write`. After a `save`, `Data.LastPlayed`
   (wall clock, WorldInfo.java:96) and `Data.SizeOnDisk` (sum of region growth, which
   depends on the deflate output) are removed from the tree and listed in `masked`.
2. `{"e":"world_info","RandomSeed","SpawnX","SpawnY","SpawnZ","Time","LevelName","version","rainTime","raining","thunderTime","thundering","dimension","player"}`:
   `new WorldInfo(Data)` (WorldInfo.java:22-40) read back through its getters; `player` is
   whether a `Player` compound exists.
3. Per region file `region/*.mcr`, in file-name order: `{"e":"region_header","name","count","present":[i, …],"timestamps_match","dbg":{"len","locations":[1024 ints]}}`.
   `present`: indices `x + z * 32` with a non-zero location entry (RegionFile.java:251-253);
   `timestamps_match`: the timestamp table (wall-clock seconds, RegionFile.java:232) is
   non-zero exactly at those indices. Sector offsets, sector counts and the file length
   depend on the deflate output and are in `dbg` only (never compared).
4. Per chunk of the op's / scenario's `chunks` (default: every present chunk, in region-file
   then index order): `{"e":"chunk_nbt","x","z","root":<canonical tree>}`, the stored
   compound read as `McRegionChunkLoader.loadChunk` reads it (McRegionChunkLoader.java:15-17;
   `Level.LastUpdate` is the world time of the save); for `world_read` only, the
   `chunk_digest` line (`"stage":"loaded"`) of the `Chunk` that loader builds
   (ChunkLoader.java:151-210, `removeUnknownBlocks`); then one line per entry of
   `Level.TileEntities` in list order (which is `chunkTileEntityMap` HashMap order at save
   time): `{"e":"tile_entity","index","id","known","class","x","y","z","size","items":[{"slot","id","count","damage"}, …]}`
   via `TileEntity.createAndLoadEntity` (TileEntity.java:45-63); `size`/`items` only for an
   `IInventory`, non-empty slots in slot order; `known: false` (and nothing more) when the id
   has no mapping. A requested chunk that is not in its region file gives
   `{"e":"chunk_absent","x","z"}`.

`world_read`:

```json
{"name":"testworld","kind":"world_read","tier":"S","dir":"world","chunks":[{"x":0,"z":-4}]}
```

`dir` is a sidecar directory in the scenario directory holding a save (`level.dat`,
`region/`); it is an input, never rewritten (`regen.sh` only replaces top-level files). The
oracle copies it to a temp directory (opening a save writes `session.lock`), opens it with
the loading constructor `World(SaveOldDir, "harness", 0, null)` (World.java:149-208; the
dimension and seed come from `level.dat`) and emits the lines 1–4 above at `t = 0`, with
nothing masked. No `seed`: the harness streams use 0, and the output does not depend on it.

`save/testworld/world` holds the owner's `testWorld` (seed 123, not Mojang content):
`level.dat` and only `region/r.0.-1.mcr` (1.2 MB; the whole `region/` is 3.6 MB), the
region containing chunk (0, −4) with the chest (PLAN_v4 §4.4 item 4). The oracle reads a
`Chest` at (9, 64, −61) holding 2 planks (id 5, slot 0) and 4 sticks (id 280, slot 1).

Rust side (WP-1.9b): `crates/mc-golden/src/world_read_driver.rs` (`drive_in(fixture_dir)`)
copies the sidecar directory into a `MemoryStorage` as `world/…`, opens it with
`World::new_from_save` on a `SaveOldDir` (seed 0, no provider) and emits lines 1–4 through
`emit_tier_s`, which the tick driver's `save` op shares (`"save_format": "mcregion"` builds the
world on a `SaveOldDir` over an in-memory storage; `World::try_save_world(true)`, then
`emit_tier_s` masked). The save code's clock is a fixed `HARNESS_CLOCK_MILLIS` (non-zero
seconds, so `timestamps_match` holds; `LastPlayed` and the timestamps are never compared).
One class difference is bridged in the tick driver, as `trace_kind` bridges the name: the
oracle's `HarnessPlayer` extends `EntityPlayer`, so its `level.dat` player has no `Score`
(`EntityPlayerSP.writeEntityToNBT`, EntityPlayerSP.java:98-101); the port's player is
`EntityPlayerSP`, and `HarnessSaveHandler` drops `Score` before `level.dat` is written. Tests:
`crates/mc-sim/tests/golden_save.rs` (the fixtures, PLAN_v4 §4.4 item 4, write/re-read, the
D5 import refusal, the `level.dat` shapes of `nbt/level-dat-shape` and
`nbt/testworld-level-dat`, and the cross-read both ways through
`tools/oracle/java/harness/WorldReadDir.java`: `read <parent>` prints what the vanilla loaders
read from `<parent>/world` exactly as a `world_read` scenario, `write <parent>` saves a small
vanilla world there; run when `build/oracle` exists).

### Committed Tier B/C/S scenarios

Generated terrain, seed 123 (ocean around the origin; grass at y = 66 near (−26, −58)):
`tick/empty-world-20` (no player: no random ticks; world time, RNG state, scheduled ticks,
chunk (0,0)), `tick/water-flow` (flowing water 3 above grass, column and box dumps),
`tick/sapling-with-player` (random ticks and peaceful mob spawning need the player; 400
ticks), `tick/bonemeal-sapling` (`use_item` bone meal grows a tree; `set_time`,
`set_block_silent`), `entity/pig-falls` (into the sea at (8.5, 90, 8.5); `"all"` includes the
generated sand that falls in the first ticks), `entity/item-drop`, `entity/arrow-flight`,
`entity/pig-attacked` (player punch and a direct hit; `set_rotation`, `set_pos`),
`save/roundtrip-seed-123`, `save/testworld`, `tick/dungeon-chest-save` (WP-1.7, Tier S:
preload the 3×3 around chunk (1, 1) and save it at t = 0; its `TileEntities` hold the
dungeon's spawner and the two chests' loot drawn on the generator's `Random`).

Empty world (`"world": "empty"`), stone platform at y = 60 over −8…8 with a one-high rim at
y = 61, built by `fill` at t = 0: `tick/flat-water-flow` (still water, id 9: it never flows,
see below), `tick/flat-flowing-water` (id 8), `tick/flat-sand-falls` (an
`EntityFallingSand` appears and lands), `tick/flat-torch-light` (block light),
`entity/flat-item-drop`, `entity/flat-pig-falls`. They exist so a port can verify one block
and one entity before it has terrain generation.

Entity base physics (WP-2.1: `Entity.onEntityUpdate` / `moveEntity` through the two
transcribed non-living kinds, `EntityItem` and `EntityFallingSand`), empty world, seed 123,
`"mob_spawning": "none"`, the same platform; `entity_state` every tick (`EntityItem`
positions are box centres, so an item resting on the platform has `posY` 61.125):

- `entity/item-in-water`: a two-deep still-water pool; an item falls in (the splash and its
  particles draw from `Entity.rand`, Entity.java:205-236) and one starts in the water (no
  splash on the first update). `rng_state` every tick.
- `entity/item-in-lava`: a still-lava pool; an item falls in (`setOnFireFromLava` and the
  burn of `moveEntity` kill it), one rests beside it, and one starts low in the lava where
  only `EntityItem.onUpdate`'s lava bounce and the burn reach it. `rng_state` every tick.
- `entity/item-in-fire`: a fire block on netherrack; the item burns (`isBoundingBoxBurning`
  → `dealFireDamage`, the fire counter and its `% 20` hit). `rng_state` every tick.
- `entity/item-in-web`: webs slow a falling and a sliding item (`isInWeb`, Entity.java:303-311).
- `entity/item-steps-up`: a flow pushes an item against a one-high ledge and a sliding item
  hits a block (step height 0: no step-up); falling sand lands on a half slab and drops as an
  item. `rng_state` every 10.
- `entity/item-out-of-world`: an item falls past the platform edge and is killed below
  y = −64 (`entity_absent` afterwards).
- `entity/falling-sand-on-cactus`: falling sand lands on a cactus (`Entity.attackEntityFrom`'s
  base body) and an item on another loses health each tick. `rng_state` every 10.
- `entity/item-collision-pair`: two overlapping items do not push each other (`EntityItem`
  never calls `applyEntityCollision`; that and `isEntityInsideOpaqueBlock` are unit-tested in
  `crates/mc-sim/tests/entity_base.rs`).

The step-up branch of `moveEntity` (`stepHeight` > 0) needs a living entity and has no
fixture yet.

Fluids (WP-1.6a: `BlockFluid`, `BlockFlowing`, `BlockStationary`), empty world, seed 123,
`"mob_spawning": "none"`, on the same platform unless stated; all dump `block_box` of the
pen and `rng_state` every tick:

- `tick/flat-lava-flow`: flowing lava (id 10) at (0, 61, 0), 300 ticks, source removed at
  t = 100. Pins lava's overworld level step 2 (BlockFlowing.java:23-26), tick rate 30
  (BlockFluid.java:187-193) and, while it drains, the `World.rand.nextInt(4)` stall on every
  level rise (BlockFlowing.java:59-62).
- `tick/flat-lava-flow-nether`: the same with `"dimension": "nether"`, 500 ticks, source
  removed at t = 240: level step 1, so it spreads 7 blocks; the tick rate stays 30 (b1.7.3
  has no dimension-dependent tick rate).
- `tick/flat-water-lava-mix`: water and lava sources meeting, a lava source reached by water,
  a flower washed out by lava and still lava under a cup that gets water above it at t = 20.
  Pins `checkForHarden` (obsidian at level 0, cobblestone at 1..4, BlockFluid.java:247-284),
  `flowIntoBlock`'s fizz for lava (BlockFlowing.java:116-130) and `triggerLavaMixEffects`'
  2 `World.rand` and 16 `Math.random()` draws (BlockFluid.java:286-293).
- `tick/flat-lava-water-fizz`: a lava source with still water placed silently 4 blocks away
  on two sides; the level-6 lava that reaches the water fizzes without hardening
  (BlockFluid.java:271-279), wakes the water (`setNotStationary`) and is fizzed again as the
  water wraps it. The ring keeps the water off the inner lava, so nothing hardens and every
  fizz in the `rng_state` stream comes from this path.
- `tick/flat-water-drain`: water sources that settle into still water, a source pair over
  stone that fills the cell between them (BlockFlowing.java:51-57), and two sources removed
  (t = 40, 80): `setNotStationary` (BlockStationary.java:23-30) and the levels decaying.
- `tick/flat-water-spread-holes`: a two-layer platform (y = 59, 60) with one-deep pits; the
  flow-direction choice of `getOptimalFlowDirections` / `calculateFlowCost`
  (BlockFlowing.java:132-218) with tied, unequal, adjacent and out-of-range (depth 4) pits.
- `tick/flat-water-blockers`: the two-layer platform with a ladder, a reed (on dirt beside a
  capped water pit), a wooden door, an iron door and a sign post in the path of a water source,
  and a ladder set into the floor. Pins `blockBlocksFlow`'s id list (BlockFlowing.java:220-231):
  the ladder and the reed are not solid materials, yet the water neither washes them out nor
  falls into the floor ladder; no item is dropped (`rng_state` never moves).
- `tick/flat-water-item-push` (Tier C): water washes a flower out as an item
  (`flowIntoBlock` → `dropBlockAsItem`) and the flow pushes the `EntityItem`
  (`velocityToAddToEntity` / `getFlowVector`, BlockFluid.java:98-185); `entity_state` every
  tick.
- `tick/flat-lava-fire`: still lava filling the pen, planks above part of it and a player
  beside it (random ticks); `BlockStationary.updateTick` climbs and sets fire next to the
  planks (BlockStationary.java:32-56), and `BlockFire` takes it from there
  (`onBlockAdded`'s scheduled tick, then `updateTick`).

Lighting (WP-1.3), empty world, seed 123, only blocks that are inert here (stone, glass,
glowstone; ice, leaves and still water placed silently and without a player). They use the
existing `block_box`, `block` and `chunk_digest` dumps; `rng_state` at the end ties them to
the seed for `check-sensitivity.sh`:

- `light/skylight-column`: a 9×9 stone slab at y = 60 (`fill`, t = 0); a 4-high pillar on it
  at (2, 61..64, 2) placed at t = 5 and removed top down at t = 20 (`set_block`). The slab
  queues more light updates than one `updatingLighting` call runs (the port's replay:
  631 left after t = 0, empty after t = 4), so the per-tick `block_box` dumps at t = 0…3 show
  that the queue does not drain in one tick (they do not pin the exact budget: 498 or 500
  updates per call still pass here); `chunk_digest` with `full` on chunk (0, 0) at the end.
- `light/blocklight-glowstone`: glowstone at (0, 62, 0) in a stone pen with a partition and a
  still-water wall (`set_block_silent` / `fill` of id 9), placed at t = 0 and removed at
  t = 20; propagation and attenuation through air and water, and the light's removal. This is
  the fixture that pins the exact 499-per-call budget: with 498 or 500 updates per call its
  t = 0 `blocks_full` block light differs.
- `light/chunk-border`: a stone roof over x = 14..18 at y = 64 and glowstone at (15, 62, 6):
  sky and block light across the chunk (0, 0) | (1, 0) edge; digests of both chunks (`full`
  for (1, 0)).
- `light/occlusion`: 3-high columns of ice, leaves, still water and glass and a leaves canopy
  under open sky; `block_box` dumps per column.

World block access and ticks (WP-1.5), empty world, seed 123. They need no block or
entity code beyond the base `Block` behaviour, `BlockSand` (sand and gravel) and
`EntityFallingSand`, and do not dump `entity_state` (the player's own update is Phase 2):

- `tick/random-ticks-stone-platform` (`"mob_spawning": "none"`): a player standing on a
  17×17 stone platform at y = 60 with a sand column, a gravel column, glowstone, glass and
  obsidian, 200 ticks; a gravel block set in the air at t = 50 falls. `rng_state` every
  tick pins `World.rand` (6000 `randomDisplayUpdates` draws a tick, the lightning and
  snow rolls of 361 chunks) and the random-tick LCG; `scheduled_ticks` (full) and
  `world_time` every 10, `block_box` of the platform at the end.
- `tick/spawning-draws` (`"mob_spawning": "default"`, difficulty 0): the stone platform
  with the sand column and glowstone of `random-ticks-stone-platform` (no gravel, glass,
  obsidian or falling gravel), with natural spawning on for 100 ticks. No grass and no water, and every spot with a normal
  cube below is within 24 blocks of the player, so `performSpawning` never reaches a mob
  constructor but makes all its `World.rand` draws (per eligible chunk: the weighted pick,
  the spawn point, 72 pack draws where the point is air) and loads the chunks it probes;
  `rng_state` every tick.
- `tick/ray-trace-platform` (`"mob_spawning": "none"`): `ray_trace` dumps against stone,
  glass, a still water source and still water at meta 3 (straight, diagonal, through glass,
  along the water row with and without `stop_on_liquid` and in both directions, from inside
  a block, from below, ending in the start block) and `power` dumps on stone and air (all
  false: no power source is transcribed yet, so they pin the traversal only).
- `tick/weather-time`: no chunks, no player, 2000 ticks; `set_time` 23990 (across a day
  boundary), 0 at t = 40 and 11950 at t = 1000 (dusk, where `skylightSubtracted` changes);
  `world_time` and `rng_state` every 10 ticks. Pins the clock, the autosave cadence and
  `updateWeather`'s first-tick timer draws (both timers start at 0); nothing else draws.
- `tick/weather-snow` (`"mob_spawning": "none"`): a player on a 32×16 stone platform at
  y = 60 over chunks (61,-13) and (62,-13), where seed 123's biomes are Taiga
  (`getEnableSnow`) on 431 columns and Forest/Swampland (rain) on 81; 400 ticks.
  `set_weather` at t = 0 (rain in 20, thunder in 30 ticks) and t = 300 (rain off in 3,
  thunder off in 5) drives all four `updateWeather` toggles and the four timer re-draws
  (raining 12000 + nextInt(12000), thundering 3600 + nextInt(12000), dry
  12000 + nextInt(168000) for both) and the strength ramps; once `isRaining`, the snow roll
  places snow layers (78) on Taiga columns only. `weather` and `rng_state` every tick,
  `block_box` of the platform and the layer above at t = 150 and the end.
- `tick/weather-ice` (`"mob_spawning": "none"`): the same platform with still water at
  every even x and even z (stone on four sides and below, so a freeze notifies no block
  override), default weather (no rain within 600 ticks; even under rain
  `BlockSnow.canPlaceBlockAt` refuses water, which is not an opaque cube); the snow roll
  freezes water on Taiga columns only. `weather` every 10, `rng_state` every tick, `block_box` at t = 299 and the
  end.
- `tick/use-item-noop` (`"mob_spawning": "none"`, WP-1.6 prep): the stone platform and a
  player; `use_item` on the top face of a platform block at t = 1 with an empty hand and at
  t = 2 holding a stick (item 280, a plain `Item`: `onItemUse` false, `onItemRightClick`
  returns the stack, Item.java:155-165). Both lines are `placed` false, `used` false, `stack`
  `null` / `{280,1,0}`; `rng_state` every tick shows the op draws nothing. The Rust driver
  records `Item.onItemUse` / `Item.onItemRightClick` as gaps (item behaviour, WP-2.6) and
  takes the base results, which are what a plain item does
  (`crates/mc-golden/src/tick_driver.rs`, op table). The held stack is passed as its hotbar
  slot, so the in-place mutations of `onItemUse` / `onItemRightClick` land in the slot as on
  Java's shared object, and `sendUseItem`'s identity test is an explicit result (`use_item`
  docs there).

Redstone (WP-1.6 task c), empty world, seed 123, the stone platform at y = 60; every
`scenario.json` has a `comment` with the layout:

- `tick/redstone-wire-line`: a torch and 15 wires (levels 15..1), and a second line over a
  step up and down (`calculateCurrentChanges`' diagonal connections), placed silently and by
  `set_block`; both torches removed at t = 10. `block_box`, `power` and `scheduled_ticks`
  every tick. The `blocksNeedingUpdate` `HashSet` order decides the notification order.
- `tick/redstone-removal` (tier C): the removal and drop paths. A wire removed from a
  powered line (`BlockRedstoneWire.onBlockRemoval` propagating at the air cell); the
  platform block removed under a powered repeater, a wire and an on lever, and the block
  behind a pressed button (each block's `onNeighborBlockChange` drop path: `dropBlockAsItem`
  draws `World.rand`, then `setBlockWithNotify` 0 and the `onBlockRemoval` notifications);
  a pressed button and a pressed wooden plate removed by `set_block` 0 (`onBlockRemoval`
  while pressed; their pending updates later find air); a pressed plate whose support is
  removed. `entity_state` (the dropped items), `power`, `scheduled_ticks`.
- `tick/redstone-torch-inverter`: a lever (switched by `set_block` with meta 13 / 5) into a
  block with a torch on it, and a one-torch clock whose wire runs back into its block: eight
  turn-offs within 100 ticks burn it out (`torchUpdates`, the `random.fizz` draws at t = 30),
  a neighbour change at t = 130 relights it after the history expired.
- `tick/repeater-delays`: four repeaters (delays 1-4) in a row behind a torch, a one-tick
  lever pulse into a delay-4 repeater, a repeater facing -x; `power` in front of each.
- `tick/button-lever`: `use_item` (empty hand) on a lever, a button (released by its
  `updateTick` 20 ticks later; a second press while pressed changes nothing) and a repeater
  (delay cycling); `ray_trace` into the lever and the button (bounds while pressed).
- `tick/pressure-plate` (tier C): items fall onto a wooden plate (pressed from inside the
  item's `moveEntity`, rechecked every 20 ticks, released after `set_pos` moves the item
  off) and a stone plate (ignores items); `entity_state`, `power`, `ray_trace`.
- `tick/redstone-ore-touch`: `use_item` on redstone ore (glows, `sparkle` draws from
  `World.rand`), a layer of glowing ore whose random ticks revert it and whose
  `randomDisplayTick` draws `World.rand` inside `randomDisplayUpdates`; an item landing on
  ore does not trigger `onEntityWalking` (`EntityItem.canTriggerWalking` is false).
- `tick/torch-placement` (tier C): torches and redstone torches on every face of a block,
  on the floor, on a fence and in mid-air (dropped), the supports removed at t = 10 and
  t = 20; a redstone torch under a block powering it and an inverter on that block;
  `ray_trace` against the torches (`BlockTorch.collisionRayTrace` bounds).
- `tick/redstone-item-placement`: torches, levers, buttons, a repeater and redstone placed
  from items (`onBlockPlaced`, `onBlockPlacedBy`). Passes since WP-2.6 (`Item.onItemUse`).

Fire, plants, crops, leaves, snow and ice (WP-1.6 task b), empty world, seed 123,
`"mob_spawning": "none"`, a standing player for random ticks; `rng_state` every tick in all
of them. Random ticks and the snow roll pick positions from the LCG and `World.rand`, so
which blocks change is deterministic. No block in them notifies still water (the fluids are
another task), and none dumps the player's `entity_state` (WP-2.5):

- `tick/fire-spread-planks`: fire set on a plank floor (against a leaves hedge, between wool
  and a plank wall) and in the gap to a plank pillar; 300 ticks of `BlockFire.updateTick`
  (scheduled every 40 and random), `tryToCatchBlockOnFire`, air ignition next to flammable
  blocks; `block_box` and `scheduled_ticks` every 10.
- `tick/fire-burnout-stone`: fires of ages 0, 2, 3, 4, 14 and 15 on stone burn out; on
  netherrack forever; on obsidian `onBlockAdded` asks `BlockPortal.tryToCreatePortal` (no
  frame: false); on glass and in the
  air removed at once.
- `tick/fire-farmland-rain`: the rain branches (`canBlockBeRainedOn`, World.java:2449-2459;
  every platform column is a rainy biome at seed 123). `set_weather` starts rain at t = 59
  (`isRaining` from about t = 80): fire on planks under open sky goes out
  (BlockFire.java:61); fire on a plank floor under a stone roof with one hole burns on;
  fire on netherrack set at t = 120 among tall grass on dirt removes the rained-on grass
  instead of lighting it (BlockFire.java:119-127); dry farmland under open sky becomes wet
  (BlockFarmland.java:37), under a stone roof turns to dirt. `block_box`,
  `scheduled_ticks` and `weather` every 10.
- `tick/fire-rain-spread`: steady rain from t = 0; nine fires on netherrack (never put out
  by rain) ringed by wool at distance 2, under a stone roof at y = 63 with one-block holes,
  1600 ticks. Holes over five of the fires block their neighbouring targets through each
  rain term of the spread loop, and the two diagonal targets on the +x side only through
  the vanilla bug that tests `(x' - 1, y', z)` with the fire's `z` (BlockFire.java:97);
  holes over the corner targets of the other four block those through the target's own
  term; two plain fires on wool under lone holes go out through the fire's own term
  (BlockFire.java:61). With any one rain term of `updateTick` removed, or the bug fixed,
  the Rust port fails this fixture. `block_box` and `scheduled_ticks` every 20.
- `tick/grass-spread`: grass stripes and single blocks spread to lit dirt and die under stone;
  grass in a closed box neither; a dandelion in the dark and a dead bush on dirt drop;
  mushrooms in sunlight stay until a neighbour changes (`BlockMushroom.updateTick` does not
  call `checkFlowerChange`); `set_time` 14000 at t = 600 stops the spreading.
- `tick/crops-growth`: a 9×9 wet farmland field kept wet by still water in a stone ring, crops
  growing by `getGrowthRate`; dry farmland reverting to dirt unless crops stand on it; ripe
  crops on dirt dropping wheat and seeds on a neighbour change (`entity_state` of the drops);
  stone on farmland.
- `tick/sapling-grow`: 1024 oak, spruce and birch saplings (most with the ready bit set) over
  four chunks; 2000 ticks grow `WorldGenTrees`, `WorldGenBigTree`, `WorldGenTaiga2` and
  `WorldGenForest` trees, and saplings whose tree does not fit are put back;
  `chunk_digest` of the four chunks every 100, the tree volume at the end.
- `tick/leaves-decay`: a trunk removed under its canopy (`BlockLog.onBlockRemoval` marks the
  leaves within 4) and the decay search over `adjacentTreeBlocks`; a second tree's marked
  leaves find their log and stay.
- `tick/snow-ice`: the Taiga platform of `weather-snow` with rain: snow layers on stone and on
  leaves (`isOpaqueCube` is `!graphicsLevel`, true in the oracle), none on glass or ice;
  isolated water cells freeze; around a glowstone ice melts to still water and a snow layer
  melts, a snow block (opaque: block light 0) stays; a snow layer on glass breaks on a
  neighbour change. With `BlockSnow.canPlaceBlockAt` transcribed, `weather-snow` and
  `weather-ice` still pass unchanged.
- `tick/cactus-reed-growth`: cacti on sand and sugar cane beside still water at metadata 14
  and 15 grow a block per 16 random ticks up to three high; a cactus with a solid neighbour
  and a cane without water break (drops in `entity_state`); every growth step's
  `setBlockMetadataWithNotify` schedules the sand below (`scheduled_ticks`).

Doors, beds, ladders, cake, portal, fences, stairs, slabs, workbench and locked chest
(WP-1.6 task f), empty world, seed 123, the stone platform at y = 60; the scenario
`comment`s give the details. A player only where `use_item` or random ticks need one, and
then `entity_state` lists the drops' ids (the port's player does not update yet):

- `tick/door-toggle`, `tick/trapdoor-toggle`, `tick/bed-place-and-break`, `tick/cake-eat`,
  `tick/activate-workbench-stairs`: `blockActivated` through `use_item` (door halves,
  trapdoor bit 2, the bed's redirect and stale occupied bit, the cake at full health, the
  workbench's `true` and the stairs' model-block `false`), plus `ray_trace` through doors
  and trapdoors (`collisionRayTrace` with the bounds from the metadata).
- `tick/door-support-removed`, `tick/ladder-placement`, the support removals in
  `trapdoor-toggle`, `bed-place-and-break` and `cake-eat`: `onNeighborBlockChange` drops.
- `tick/portal-frame` (Tier C): portal blocks filled silently into obsidian frames, an
  item falling through them (null collision box), the frames broken.
- `tick/fence-stairs-step-bounds` (Tier C): falling sand and items on fences, both stairs
  orientations, slabs, a cake, a trapdoor and past a ladder (the collision boxes), and the
  slab merge of `BlockStep.onBlockAdded`.
- `tick/locked-chest-vanishes`: random ticks remove locked chests.
- `tick/portal-display`: a player next to silently filled portal blocks, so
  `randomDisplayUpdates` reaches `BlockPortal.randomDisplayTick`. That tick draws from the
  unseeded display `Random` (census, World.java:1999) and emits only particles and sounds,
  so the fixture pins only that it draws nothing from `World.rand` and changes no block;
  its own draw order is unverified.
- `tick/portal-player-collide` (WP-3.1b): the player moved (`set_pos`) into silently
  filled portal blocks and out again. `BlockPortal.onEntityCollidedWithBlock` →
  `EntityPlayer.setInPortal` drops `timeUntilPortal` from 20 to 10 on the first tick in
  the portal; the harness's `EntityPlayer` never counts it down (only `EntityPlayerSP`
  does), so `inPortal` stays false. `player_state` every tick.
- `tick/door-iron-powered` (redstone torch: doors and trapdoors opened by power, and the
  trapdoor's power branch running on its emptied cell, BlockTrapDoor.java:115-123),
  `tick/door-upper-powered` (lever: the upper half forwarding a power notification to the
  lower half, BlockDoor.java:141-143) and `tick/portal-ignite` (`BlockFire.onBlockAdded` →
  `tryToCreatePortal`) need the redstone and fire tasks, both in.
- `tick/bed-nether` needs `Explosion` (WP-1.8, in). `tick/block-item-placement`
  (`ItemBlock` and the other placing items) passes since WP-2.6.

Tile entities, container blocks and explosions (WP-1.8), empty world, seed 123, the stone
platform at y = 60 (none needs a player unless it says so):

- `tick/chest-double-and-break` (player): two chests side by side, `use_item` on one with an
  empty hand (`BlockChest.blockActivated`), two chest placements next to the pair refused by
  `BlockChest.canPlaceBlockAt`, then both broken with `set_block` air (an empty inventory
  scatters nothing). `rng_state` and `block_box` every tick.
- `tick/note-jukebox-activate` (player): `use_item` on note blocks (`BlockNote.blockActivated`:
  pitch and `playNoteAt`, true) and an empty jukebox (`blockActivated` false).
- `tick/furnace-idle`, `tick/dispenser-idle`: furnaces / dispensers placed next to stones,
  facing from `setDefaultDirection` / `setDispenserDefaultDirection`, one removed, a neighbour
  change; a lit furnace stays lit without fuel.
- `tick/dispenser-dispense-item`: a dispenser powered by a redstone torch placed with
  metadata 1 next to it: `onNeighborBlockChange` schedules, `updateTick` dispenses from an empty
  inventory (no op can fill one): the 1001 click. Needs the redstone torch (WP-1.6c, in).
- `tick/explosion-crater`: the `explode` op (size 4) over dirt on stone with a sand column, an
  obsidian wall and three `EntityItem`s (one destroyed, one shielded, one out of range): rays,
  `destroyedBlockPositions` order, entity damage and knockback, drops, falling sand.
- `tick/explosion-fire`: a flaming `explode` (size 3): the fires `doExplosionA` sets are cleared
  again by `doExplosionB` (no fire is left; their scheduled ticks are). Needs `BlockFire` (WP-1.6b, in).
- `tick/mob-spawner-delay` (player): a mob spawner counting down its 20 ticks (three `World.rand`
  draws per tick); `tick/mob-spawner-spawn` continues into the spawn attempts (the
  `EntityPig` constructor through `EntityList.createEntityInWorld`, the placement draws, an
  animal's `getCanSpawnHere` failing on stone: in since WP-2.3).
- `tick/sign-pops-off`: wall signs and a sign post losing their support (`BlockSign.onNeighborBlockChange`
  drops an `Item.sign`), a wall sign with metadata 0 dropping at the first neighbour change.
- `save/tile-entities` (Tier S, player): one tile entity of each class in chunk (0, 0), the note
  block clicked twice, then `save`: the `chunk_nbt` `TileEntities` list and the `tile_entity` lines
  pin every `writeToNBT` shape and the `chunkTileEntityMap` order.

Not pinned by any fixture: the lightning strike path of `updateBlocksAndPlayCaveSounds`
past its `nextInt(100000)` roll (World.java:1920-1930; on the weather-snow platform a strike
would need the one-in-100000 roll on a platform column of a rainy biome, and the bolt it makes
draws from an unseeded `rand` in the oracle, census §3; the bolt itself is pinned by
`entity/lightning-strike` below), and so `canBlockBeRainedOn`'s biome test
(World.java:2449-2460). A roll that does succeed on an empty column stops at
`canBlockSeeTheSky` (y = -1), before the biome test.

Non-living entities (WP-2.4, task 1), empty world, seed 123, `"mob_spawning": "none"`, the
stone platform unless stated, `rng_state` and `entity_state` every tick:

- `entity/arrow-flight` (above, generated terrain) and `entity/arrow-in-ground-pickup`: a
  spawned arrow flies (rotation from the motion on its first update, `rayTraceBlocks_do_do`
  with `ignoreNoCollision`), sticks (0.05 back-off, `arrowShake` 7), a player is moved onto it
  (a spawned arrow does not belong to a player, so `onCollideWithPlayer` never reaches the
  WP-2.5 inventory), and when its block is removed it falls out (three `rand` floats) and
  sticks again.
- `entity/tnt-explode`: a redstone torch primes a TNT block (fuse 80, the `Math.random()`
  kick), the fuse runs out, size-4 explosion with the TNT in the entity pass; it primes a
  second TNT block (`World.rand` fuse) and knocks back or destroys items.
- `tick/tnt-by-fire` (above): fire primes the TNT.
- `entity/snowball-egg-flight`: two snowballs and an egg (harness kind `Egg`) into a wall and
  onto the floor; the egg's `nextInt(8)` hatch roll fails with seed 123 (hatching:
  `entity/chicken-fall-hatch`, WP-2.3).
- `entity/painting-art`: four paintings by the position constructor (art draws replayed on
  the seeded `rand`); one loses its wall block and drops at its 101st update, one is hit by a
  snowball (`EntityPainting.attackEntityFrom`).
- `entity/fireball-explode`: a fireball (harness kind `Fireball`) falls onto a **glass**
  platform and explodes flaming; glass is not an opaque cube, so the explosion's clock-seeded
  `Random` is never drawn (census §2). The fire placement of an entity's flaming explosion is
  therefore not pinned (the `explode` op's `tick/explosion-fire` pins the code with a seeded
  `Random`).
- `entity/lightning-strike`: a bolt (harness kind `LightningBolt`, difficulty 0), its thunder
  draws, re-strikes that set fire, an item in range destroyed by `onStruckByLightning`; also
  `weather` every tick.
- `entity/fishing-hook` (player, 600 ticks): a rod given through a workbench `use_item`, then
  the harness kind `FishingHook`; the bobber flies into a 5-deep pool, floats and draws its
  bite rolls. `catchFish` needs the rod's right click (WP-2.6) and is unit-tested only
  (`crates/mc-sim/tests/non_living_entities.rs`).

Living entities (WP-2.2: `EntityLiving`, `EntityCreature`, `EntityAnimal`, `EntityPig`,
`Pathfinder`), empty world, seed 123, `"mob_spawning": "none"`, the stone platform unless
stated, `rng_state` and `entity_state` every tick (`entity/flat-pig-falls` and
`entity/pig-falls` above are the pig's first two):

- `entity/pig-wander` (600 ticks): a pig in a fenced pen, half grass: the idle AI's draws,
  `updateWanderPath` (`getBlockPathWeight` 10 on grass), `getEntityPathToXYZ` paths, walking
  and jumping (`moveEntityWithHeading` on the ground), the living-sound timer.
- `entity/pig-drowning`: a pig under a stone roof in a 2-deep pool: swimming, air down to
  -20, the bubbles and 2-point hits every 20 ticks, death, the raw porkchop drop; a second
  pig on dry stone keeps its air.
- `entity/pig-burning`: three pigs in fire pits on netherrack: fire contact and the 10-tick
  hurt-resistance window, death, cooked porkchops (`fire > 0`); the pig spawned inside the
  fire never gets `fire = 300` (Entity.java:522-532), the two that fall in do.
- `entity/pig-fall-damage`: pigs dropped 20 and 8 blocks: `EntityLiving.fall` (17 points:
  dead, drops; 5 points: survives).
- `entity/pig-knockback`: the `attack` op's direct branch on a pig, by another pig, by itself
  (the `Math.random()` direction loop), with no attacker (`Math.random()` `attackedAtYaw`),
  inside and after the resistance window, to death; a hit on the dying pig returns false.
- `entity/pig-ladder`: a pig falls onto a ladder column: the ladder clamps of
  `moveEntityWithHeading` (`fallDistance` reset, no fall damage).

Not pinned by these: `faceEntity` toward a player (it reads the player's `getEyeHeight`,
`EntityPlayer`'s override, 0.12, transcribed with WP-2.3b, whose monster fixtures below pin it
and `getPathToEntity`, as does `entity/wolf-begs-bone`). Paths to an entity (`getPathToEntity`) are also pinned
since WP-2.3 by the wolf fixtures below (the wolf is the only b1.7.3 animal with a target).
`Pathfinder` is also unit-tested on hand-built worlds (`crates/mc-sim/tests/pathfinding.rs`). The `entity_state`
dump has none of `EntityLiving`'s body yaw and walk state (`renderYawOffset`,
`field_9361_v`/`9362_u`/`9360_w`/`9359_x`, `legYaw`/`legSwing`), nor `randomYawVelocity`,
`entityAge`, `livingSoundTime`, `heartsLife`, `field_9346_af`, `attackedAtYaw`, `air` or
`fire`: those are pinned only through the `rand`/`Math.random()` streams, health and drops,
and the body yaw and walk state by `crates/mc-sim/tests/living_entities.rs`.

Animals and the wolf (WP-2.3 task 1: `EntitySheep`, `EntityCow`, `EntityChicken`,
`EntityWolf`), empty world, seed 123, `"mob_spawning": "none"`, a 16×16 stone platform over
chunk (0, 0) fenced at y = 61, `rng_state` and `entity_state` every tick unless stated. The
`interact` op's `item_id` puts the stack into the player's hotbar slot first; without one the
slot keeps what it holds. Where a player is present it stands outside the fence, beyond the
animals' 8-block look range and out of reach of the drops (its look-at target needs
`EntityPlayer.getEyeHeight`, its item pickup `EntityPlayer.onLivingUpdate`: WP-2.5):

- `entity/sheep-shear` (player, `save`): shears on a sheep (2–4 wool, three `rand` nudges each,
  the shears' damage), again on the sheared sheep (nothing), on a second sheep; the save's
  `Sheared`/`Color`; a sheared sheep dies without a drop, an unsheared one drops its wool.
  Sheep are white: only `SpawnerAnimals` draws `getRandomFleeceColor` (unit-tested in
  `crates/mc-sim/tests/animals.rs`).
- `entity/cow-milk` (player looking up, so the milk bucket's `sendUseItem` ray trace misses):
  an empty bucket becomes milk; a stack of 3 buckets becomes one milk bucket (vanilla bug:
  two buckets lost); other items do nothing; hits, a death and the leather drop.
- `entity/chicken-fall-hatch`: a chicken falls 14 blocks slowed by its wings and lands
  unhurt (`EntityChicken.fall` is empty); 59 eggs land, three hatch (one of them four
  chickens) with seed 123; `entity_state` of the chickens and the hatching eggs only.
- `entity/chicken-eggs` (7300 ticks): three chickens lay their first eggs at t = 6264, 6781,
  7246 (the constructor's egg timer, redrawn by the oracle's join hook on the seeded stream,
  which the port's seeded constructor reproduces); `rng_state` every 10 ticks,
  `entity_state` every 100 and around each egg.
- `entity/wolf-hunts-sheep` (450 ticks, `entity_state` every 2): b1.7.3 wolves hunt sheep
  (a `World.rand` 1-in-100 roll, a random sheep within 16 blocks): paths to the target,
  leaps, bites of 2; both sheep die and drop wool.
- `entity/wolf-pack-retaliates` (`save`): a pig hits a wolf (damage halved); both wolves target
  the pig, without anger (not a player), and kill it; a hit with no attacker neither halves nor
  retaliates; the save's `Angry`/`Sitting`/`Owner`.
- `entity/wolf-rain-shake`: a wolf in the rain (`set_weather`) is wet; after the rain it
  shakes for 40 ticks, standing still (`isMovementCeased`), with the shake sound and splash
  draws; `weather` every tick.
- `entity/wolf-bone-refused` (player): two bones refused (smoke draws), then stone (false).
  The interact ticks are chosen so that the taming rolls fail with seed 123 and with the
  flipped seed of `check-sensitivity.sh`: a taming would stop the oracle run (below).
- `entity/wolf-begs-bone` (player 5 blocks away; passes since WP-2.3b transcribed
  `EntityPlayer.getEyeHeight`): the wolf picks the player as its look target and, seeing a
  bone in hand, keeps facing it (`looksWithInterest`, `numTicksToChaseTarget = 10`).

Not pinned: the tamed wolf. The oracle's `HarnessPlayer` has a null `username` (only
`EntityPlayerSP` copies the session's), so a successful taming throws
`NullPointerException` in `setWolfOwner` (`DataWatcher.updateObject`), as do the owner checks
(`username.equalsIgnoreCase`); the bone fixtures' ticks are chosen so that every taming roll
fails. Following and teleporting to the owner, sitting, feeding meat and the tamed branches
of `attackEntityFrom`/`attackEntity` are unit-tested only (`crates/mc-sim/tests/animals.rs`).
Anger needs a player (or a player's arrow) as the attacker: the `attack` op's player branch is
WP-2.5.

Minecart and boat (WP-2.4, task 2), empty world, seed 123, `"mob_spawning": "none"`, the
stone platform unless stated, `rng_state` and `entity_state` every tick (of the vehicles and
their drops only where a player is present: the player's own update is WP-2.5):

- `entity/minecart-rail-loop` (400 ticks): a 5×5 rail loop with a hump (two slopes) and
  three powered rails on one torch; the cart, spawned on a slope, runs five laps (the rail
  matrix, slopes, curves, the 0.06 boost, the yaw reversal).
- `entity/minecart-powered-brake` (player): an unpowered powered rail brakes a cart to a
  stop; `use_item` on a lever powers it and the cart is pushed off the end wall (0.02) and
  boosted.
- `entity/minecart-detector-rail`: a cart crossing a detector rail powers the neighbouring
  powered rail (scheduled tick 20 turns both off after it has left); `power` and
  `scheduled_ticks` every tick.
- `entity/minecart-derail-collide`: a cart leaves a raised track, hits a wall in the air and
  lands; two carts meet head-on (`applyEntityCollision`); a cart runs past one parked 0.7 to
  the side and is blocked by its box while the vanilla bug of the cart-to-cart test
  (`dz * prevPosX`, EntityMinecart.java:676) skips every push.
- `entity/minecart-break-drops`: direct `attack`s break a plain, a chest (`type` 1) and a
  furnace (`type` 2) cart: minecart, chest and furnace items. A chest cart's cargo drop is
  not pinned (no op fills its inventory).
- `entity/minecart-furnace-push` (player): `interact` with coal fuels a furnace cart and
  pushes it away from the player; it runs into a plain and a chest cart (the furnace branches
  of `applyEntityCollision`) and jams them against a wall; `interact` on the chest cart is
  the GUI gap.
- `entity/minecart-player-ride` (player): `interact` puts the player in a cart on a slope; it
  rides down (the rider's 0.75 move and 0.997 friction); a hit wobbles it, a second breaks it
  and puts the rider off. Dismounting by `interact` is not pinned: the dismounted player then
  pushes the cart through `EntityLiving.collideWithNearbyEntities` (WP-2.2, WP-2.5).
- `entity/boat-on-water` (200 ticks): a 10×10 pool 3 deep, three boats float up (five-slice
  buoyancy); one is sent diagonally into the rim and breaks (3 planks, 2 sticks), one drifts
  into the third (boats are solid to each other; `Entity.applyEntityCollision`) and is broken
  by two direct hits.
- `entity/boat-player-ride` (player): `interact` puts the player in a boat, which is sent into
  the rim and breaks with its rider; the run ends before the player could pick up the drops.
- `entity/squid-swim` (WP-2.3, 600 ticks, no player): an 11-deep pool; one squid swims
  (`EntitySquid`'s stroke cycle and random swim direction, `isInWater`'s current push), one sits
  on a ledge (out of water it falls and tips over), one drops into the pool and is killed by a
  direct hit (1-3 ink sacs, `dropFewItems`). The constructor's `rotationVelocity` draw is the
  oracle join hook's redraw (census §3).
- `entity/squid-flowing-water` (WP-2.3, 300 ticks, no player): a 1-wide stone channel whose
  flowing-water source spreads along +x (metas 1-7, `block_box` at t = 49); two squids spawned
  in the current at t = 50 get `EntitySquid.isInWater`'s `handleMaterialAcceleration` push (a
  non-zero flow vector, 0.014 per call, twice a tick: the first call's push goes into
  `moveEntityWithHeading`).
- `tick/mob-spawner-spawn-grass` (WP-2.3, 120 ticks): `TileEntityMobSpawner`'s successful spawn
  on a lit grass pad (two pigs join at t = 20: `entityJoinedWorld`, the 20 particle triples,
  `spawnExplosionParticle`, `updateDelay`, the loop going on after a spawn). The player stands
  12.5 blocks above the spawner, out of the pigs' 8-block look range (`faceEntity` would read
  `EntityPlayer.getEyeHeight`, WP-2.5).
- `tick/mob-spawner-cap` (WP-2.3, 30 ticks): six pigs spawned inside the spawner's box; at t = 20
  the first try constructs a pig, counts 6 and takes `updateDelay` and `return` (no placement
  draw).
- `tick/natural-spawn-squid` (WP-2.3, player, difficulty 0): `SpawnerAnimals.performSpawning`
  with mobs that join: a pool more than 24 blocks from the player and the spawn point fills
  with squid packs (construction through `EntityList`'s table, `setLocationAndAngles`,
  `EntityWaterMob.getCanSpawnHere`, `entityJoinedWorld`, the 4-per-pack limit and the per-pass
  gate (a pass runs while `countEntities(EntityWaterMob) <= 5 * 289 / 256 = 5`, so the world can
  exceed 5));
  nothing else is a spawn spot, so no animal constructor is reached.
- `tick/natural-spawn-pigs` (WP-2.3, generated terrain, player on the grass of
  `tick/sapling-with-player`, difficulty 0): natural animal and squid spawning end to end
  (every `SpawnListEntry` constructor: sheep, pig, chicken, cow, squid; wolves in forests).
  Passes since the WP-2.5a merge (the peaceful heal of `EntityPlayer.onLivingUpdate`), as
  does `tick/sapling-with-player`.
- `tick/sleep-spawning` (WP-2.3, player, difficulty 1): a player sleeps in a bed in a dark,
  water-roofed stone room (no monster spot outside it, so the unseeded `EntityMob.getCanSpawnHere`
  rolls of census §3 always pass); after 100 ticks `performSleepSpawning` places a skeleton by
  the bed and wakes the player. Stops before the skeleton's first arrow (unseeded heading
  jitter). Passes since the WP-2.5a merge (`EntityPlayer.sleepInBedAt` and `wakeUpPlayer`);
  the skeleton is in since WP-2.3b.

Monsters, slime and ghast (WP-2.3, task 2), empty world, seed 123, `"mob_spawning": "none"`,
`"difficulty": 1` (`EntityMob.onUpdate` and `EntityGhast.updatePlayerActionState` kill them
on peaceful), `rng_state` and `entity_state` every tick. A monster that hits the player makes
the player's own fields diverge (`EntityPlayer.attackEntityFrom`'s difficulty scaling and
`damageEntity`'s armour are WP-2.5; its knock-back is `EntityLiving`'s and matches), so those
scenarios dump the monsters only (`which`) and stop before the port's player, which takes the
unscaled damage, would die. Night is `set_time 18000` at t=0 with the mobs joining at t=1,
after `World.tick` has recomputed `skylightSubtracted`:

- `entity/zombie-chase-player` (night, 150 ticks): `EntityMob.findPlayerToAttack` (16 blocks,
  `canEntityBeSeen`), `getPathToEntity`, the walk, `faceEntity`, three melee hits 20 ticks
  apart that knock the player back.
- `entity/skeleton-shoot` (night, 300 ticks): the skeleton at a player behind a fence row:
  an arrow every 30 ticks (`attackTime`), the arrows (ids 1-12 dumped) stick in the floor or
  hit the player.
- `entity/skeleton-sunburn` (noon, no player, 600 ticks): the daylight burn roll, fire damage
  every 20 ticks, the wander, death and the arrow/bone drop.
- `entity/spider-leap` (night, 200 ticks): `EntitySpider.findPlayerToAttack` in the dark, the
  1-in-10 leap between 2 and 6 blocks, melee, and climbing the arena wall
  (`isOnLadder = isCollidedHorizontally`).
- `entity/creeper-fuse-explode` (100 ticks): the creeper slides along a fence toward the
  player, fuses (watcher byte 16) and explodes after 30 ticks (`createExplosion(creeper, …,
  3)`); `block_box` of the dirt crater at t=99. The fence keeps the explosion weak enough for
  the port's player to survive, so the random block ticks around it continue as in the oracle.
- `entity/pigzombie-anger` (day, 300 ticks, everything dumped: the player is never hit): the
  harness `FishingHook` hooks one of three pigmen, `attackEntityFrom(angler, 0)` is a player's
  hit, so every pigman within 32 becomes angry (`becomeAngryAt`'s two draws); they run to the
  fence that keeps them from the player; the angry sounds.
- `entity/slime-split` (no player, 480 ticks): a size-4 slime jumps for 400 ticks (the jump
  timer, strafe and sound draws, the landing particles), is killed with exactly its health and
  splits into four size-2 slimes, killed in turn into sixteen size-1 slimes, killed again
  (slimeball drops). The split slimes are killed on the tick after they appear, before their
  first ground tick runs `slimeJumpDelay`: their constructors draw it from the oracle's
  unseeded `Random` (census §3, a gap the join hook does not close).
- `entity/ghast-waypoints` (336 ticks): a ghast in a closed stone box, the player behind a
  dividing wall: 300 ticks of waypoints (new ones, pushes, blocked courses), then the wall is
  removed, the ghast charges and fires one fireball; the run ends before the fireball reaches
  anything, since `new EntityFireball(world, ghast, …)` draws its acceleration jitter from the
  oracle's unseeded `Random` (census §3; the fireball is not dumped).

Not pinned by a fixture (unit-tested in `crates/mc-sim/tests/monsters.rs`): the giant zombie
and `EntityList`'s `Monster`, NBT, the powered creeper (lightning) and its record drop,
`EntitySlime.onCollideWithPlayer` (called by `EntityPlayer.onLivingUpdate`, WP-2.5), and the
spawn checks (`getCanSpawnHere`: natural hostile spawning is not reproducible in the oracle).

Crafting and containers (WP-2.5), empty world, seed 123, `"mob_spawning": "none"`, a player on
a stone floor; `use_item` on a workbench (its `blockActivated` returns true, so the stack is
not used) loads hotbar slot 0 with a stack before the clicks:

- `tick/craft-ops`: `craft` ops for 23 grids: 2×2 and 3×3, a `Block` ingredient of another
  damage (any damage matches), patterns at an offset and mirrored (axe, bow, stairs), shapeless
  dyes in any cells and with 1–4 ingredients, an `ItemStack` ingredient (charcoal), the cake,
  ingots both ways, a wrong damage, wrong grids (`null`), a 2×2 shapeless recipe. The recipe
  list itself is `recipe/recipe-table` (Tier A).
- `tick/inventory-slot-clicks`: `slot_click` in the player's `ContainerPlayer`: pick up
  (left: all, right: the larger half), place (all, one), merge, swap, armour slots refusing and
  taking a helmet, crafting planks and sticks in the 2×2 grid (`SlotCrafting` pickups into an
  empty hand and onto a matching held stack), shift-clicks from the output, main inventory,
  hotbar and armour (`mergeItemStack`'s top-up and first-empty passes); `inventory` every tick.
- `tick/chest-container-clicks` (Tier S): `slot_click` in a double chest's `ContainerChest`
  (`InventoryLargeChest`) and a single chest's, shift-clicks both ways, a container switch;
  then `save`: the chests' `Items` in the `tile_entity` lines.
- `tick/workbench-crafting`: a `ContainerWorkbench` kept open across clicks: the cake (milk
  buckets left as empty buckets, `getContainerItem`), buckets (stack limit 1) clicked onto
  each other, planks shift-clicked out (one craft per shift-click) and the output picked up
  whole with a right click.
- `tick/furnace-dispenser-clicks` (Tier S): `ContainerFurnace` (input, fuel, the
  `SlotFurnace` output taken after 200 ticks of smelting, shift-clicks) and
  `ContainerDispenser` (no shift-click move); then `save`.

The player (WP-2.5a: `EntityPlayer`, the `EntityPlayerSP` overrides `HarnessPlayer` has,
`PlayerControllerSP`), empty world, seed 123, `"mob_spawning": "none"`, stone floors at
y = 60; `entity_state`, `player_state` and `rng_state` every tick unless stated. Drops are
placed beyond the player's pick-up reach (`InventoryPlayer.addItemStackToInventory` is
WP-2.5b):

- `entity/player-walk-jump` (175 ticks): `input` walks forward, jumps onto a one-high step,
  strafes, then sneak-strafes to the east edge, where `Entity.moveEntity`'s sneak guard holds
  the player; released, its remaining motion carries it over the edge.
- `entity/player-dig-stone` (240 ticks): `dig` breaks a dandelion on the first click, stone
  bare-handed (150 ticks, no drop: not harvestable by hand; the 5-tick repeat clicks; the
  5-tick wait after a break), then with a wooden and an iron pickaxe chosen by `input`
  `hotbar` (cobblestone drops, tool wear); `block_box` of the four blocks every tick.
- `entity/player-fall-damage` / `entity/player-fall-damage-armour`: two 10-block falls,
  without armour and in full leather (`damageEntity`'s 25ths and `damageRemainder`,
  `damageArmor`'s wear); the peaceful heal every 20 ticks.
- `entity/player-attack-pig`: `attack` with the player as attacker and a wooden sword: 4 + 1
  (the ground's gravity step makes `motionY < 0`), knock-back from the player, the hurt
  window, the kill (`addToPlayerScore` with `scoreValue` 0), sword wear, a hit on the corpse.
- `entity/player-death-drops` (56 ticks): a 39-block fall kills the player:
  `EntityPlayer.onDeath`, `dropAllItems` (six stacks incl. a chestplate, two `rand` draws
  each), `entity_state` and `rng_state` only; it ends before the player leaves
  `loadedEntityList` (vanilla keeps the dead player in `World.playerEntities`, which the
  port's `World` cannot hold yet).
- `entity/player-sleep` (170 ticks): `sleep` by day (`NOT_POSSIBLE_NOW`), at a far bed
  (`TOO_FAR_AWAY`), `OK`, again on the occupied bed (`null`); `set_time` to day wakes the
  player (`wakeUpPlayer(false, true, true)`, the bed becomes the spawn point); asleep again,
  `World.tick` skips to the next morning after 100 ticks; `world_time` and the bed's
  `block_box` every tick.

Item behaviour (WP-2.6), seed 123, empty world unless noted, a player with a preloaded
`inventory`, `"mob_spawning": "none"`; `use_item` and `interact` without `item_id` use the
held stack, so a fixture can use up and break what it holds. Every one dumps `rng_state`
and, where the stacks matter, `inventory` every tick:

- `tick/place-stairs-slabs-doors-beds-signs`: `ItemBlock.onItemUse` and the placing items on
  every face: stairs at four yaws (`BlockStairs.onBlockPlacedBy`), on a side face and under a
  block; slabs (a stone slab on a stone slab merges into a double slab in
  `BlockStep.onBlockAdded`, other types stack); doors (`ItemDoor`: the second of two side by
  side gets the mirrored hinge; refused on a side face and under a blocked cell), beds
  (`ItemBed`, two yaws, refused on a side face and with the head cell taken), signs
  (`ItemSign`: posts at two yaws, wall signs on four faces, refused on a bottom face, a flower
  and an occupied cell); planks on all six faces of a floating block; the sub-type items'
  metadata (wool, log, sapling, leaves with the no-decay bit, the piston's 7 then its
  facing); a snow layer replaced in place (side becomes 0); y = 127 refusing a solid block but
  not a torch; a stack of 0 refused; a block into the player's own cell refused (the player
  is in `World.checkIfAABBIsClear`'s scan; the port's player is out of its slot while the item
  runs, so `item::behaviour` checks it there).
- `tick/bucket-fill-empty`, `tick/bucket-nether-fizz` (the nether: water only fizzes, a
  `World.rand` pitch and eight `Math.random()` smoke offsets): `ItemBucket` through
  `sendUseItem`, aimed with `set_rotation`: the eye ray stops on liquid sources for the empty
  bucket (flowing water does not stop it), fill and empty water and lava, a ray crossing a
  corner into a solid block (nothing placed), milk, no hit.
- `tick/bow-shoot` (tier C): the bow takes arrows from the inventory until none are left;
  snowballs. Only the player is dumped: the projectiles' constructors draw their heading
  jitter from their own `rand` before the join hook seeds it (tools/oracle/census.md §3), so
  their flight is not reproducible in the oracle; it draws nothing from `World.rand` or
  `Math.random()`.
- `tick/bow-arrow-pickup` (tier C, WP-2.H): `EntityArrow.onCollideWithPlayer` on the player's
  own arrows (`doesArrowBelongToPlayer` from the `(World, EntityLiving)` constructor). Shot at
  pitch 60 at t = 2, the arrow sticks in the floor in front of the player at t = 3
  (`arrowShake` 7) and goes back into slot 5 at t = 11, the first player update that sees
  `arrowShake` 0; shot at pitch 0 at t = 15 into a wall out of reach, it is picked up at
  t = 30 after a `set_pos` next to the wall. As in `tick/bow-shoot` the arrows' positions are
  not reproducible, so only the player, the inventory, `rng_state` and the arrows'
  `entity_absent` lines at t = 49 are dumped; the geometry keeps the hit tick and the reach
  test far from their boundaries. The snowball and egg pickups (`EntitySnowball` /
  `EntityEgg.onCollideWithPlayer`) cannot be reached in vanilla: only an NBT-loaded
  projectile is in the ground, and its `thrower` (not saved) is null.
- `tick/flint-steel-tnt-portal`: fire on a block, beside a block, refused into stone (still
  worn), removed under a floating block (`BlockFire.onBlockAdded`), a portal lit in an
  obsidian frame, fire beside TNT; the `dig` op with flint and steel held on TNT
  (`BlockTNT.onBlockClicked` sets metadata 1, the instant break primes it); a worn flint and
  steel breaks. 200 ticks: the primed TNT explodes, the fires burn out.
- `tick/hoe-seeds-crops`: `ItemHoe` (grass from any face but the bottom with air above; dirt
  from any face whatever is above, a vanilla bug), a worn hoe breaking, `ItemSeeds` only on
  farmland's top face with air above; then 300 ticks of crop growth and farmland drying.
- `tick/food-eat-heal` (tier C): `ItemFood`, `ItemSoup` (a bowl back), `ItemCookie`, the
  golden apple capped at 20, eating at full health; the player hurt by direct `attack` ops.
- `tick/dye-bonemeal-crops` (tier C): bone meal on crops (`fertilize`), on grass (128 walks
  drawn from `Item.itemRand`), on two saplings (`growTree` with `World.rand`), on dirt
  (nothing); red dye on grass (nothing); `interact` with dyes on a sheep (`ItemDye.saddleEntity`:
  one off the stack only when the colour changes).
- `tick/sword-pickaxe-durability` (tier C): wear from `hitEntity` (sword 1, tools 2, shears
  0) and `onBlockDestroyed` (tools 1, sword 2, shears 1 on leaves and cobweb); a gold pickaxe
  breaking on stone.
- `tick/minecart-boat-painting-place` (tier C): minecarts on rails and a powered rail (refused
  on stone), boats on water (the ray stops on liquid) and on a snow layer (one block lower),
  paintings on the four faces of a lone block (only 1x1 motives fit, so the art the oracle
  draws unseeded does not show in the entity lines; a taken face fails `onValidSurface`).
- `tick/saddle-pig-ride` (tier C): a saddle used on a pig (`interact`: `EntityPig.interact`
  false, then `useItemOnEntity`), riding and dismounting, and a pig saddled by hitting it with
  a saddle (`ItemSaddle.hitEntity`).
- `tick/record-jukebox` (tier C): `ItemRecord` into an empty jukebox; a full one ejects its
  record from `blockActivated` before the item is tried.
- `tick/fishing-rod-reel` (tier C): the harness's `FishingHook` bobbers reeled in by the rod
  (`catchFish`: 2 from the ground, the rod's wear; then from the water).
- `tick/map-create-update` (`"world": "generated"`): two maps in the inventory get ids 0 and
  1 (`getMapData`, `getUniqueDataId`), the selected one is drawn around the player
  (`ItemMap.updateMapData`), fills change the sampled top blocks, the player walks;
  `map_data` of maps 0, 1 and 5 (absent).
- `tick/map-craft` (tier C): maps crafted in a workbench (`ItemStack.onCrafting` →
  `ItemMap.onCreated`): picked up with a click, the cursor's map gets the next id and a
  `MapData` centred on the player; shift-clicked out, the inventory gets a copy made before
  `onCreated` runs on the slot's own object (vanilla bug: the inventory's map keeps damage 0
  while `map_3` is made). `map_data` of maps 0-4 and the inventory.

The map files (`data/map_<id>.dat`, `data/idcounts.dat`) are not in any trace (the Tier S
lines cover `level.dat` and the regions); `crates/mc-sim/tests/item_behaviour.rs` checks the
port writes and reads them back. With `"save_format": "mcregion"` and `"world": "load"` the
harness save handler forwards them to `SaveOldDir` (the oracle's `getMapFile` is never null),
so a loaded save's `data/idcounts.dat` and `data/map_<id>.dat` are read as in Java.

Gate 2's long run (PLAN_v4 §5): `tick/testworld-6000`, `"world": "load"` of
`save/testworld/world`, the player at the spawn point (the `EntityPlayer` constructor's
spot), natural spawning off, 6000 ticks; every 500 ticks the four chunks around the spawn,
`rng_state`, `world_time` and the `inventory`; every 100 ticks the player (entity 4: the
preloaded chunks' saved entities take ids 0-3) and `player_state`; every 1000 ticks every
entity (267 lines, WP-2.H). It passes since the WP-2.5a merge, which brought
the WP-2.3 constructors of the region's 24 saved mobs (cows, skeletons, zombies, creepers,
squid, sheep, a spider) together with the loaded world. Only region `r.0.-1` is committed, and the player at the spawn
chunk (1,-4) random-ticks a 19x19 chunk square around it (World.java:1868-1965), chunks x
-8..10, z -13..5: the ones outside that region are generated and populated on both sides,
so the fixture also covers generating chunks missing from a loaded save.

Reload and Gate 2's scripted session (WP-2.G; `reload` op above):

- `tick/reload-roundtrip` (Tier S, empty world, 40 ticks): a player, a pig, an item entity and
  a chest filled by `slot_click`; sand set in the air at t = 19 schedules a tick; at t = 20 a
  `save` and the `reload`. The pig and the item come back from their chunks with new ids
  (3, 4), the player from `level.dat` (id 5), the chest from its chunk; the sand's tick is gone
  (`scheduled_ticks` count 0) and the sand stays in the air; `entity_state` and `rng_state`
  every tick, a final `save` at t = 39.
- `tick/reload-generated` (seed 123 terrain, 80 ticks): a reload at t = 30 while the
  population's falling sand is still settling and the player walks: saved chunks come back
  from the region files with their entities, unsaved ones are generated (and populated)
  again in the new load order.
- `tick/reload-nether` (empty nether world, 10 ticks): netherrack, glowstone and a chest
  set, a pig spawned, `save`, `reload` at t = 5: the world comes back as the nether (the
  provider kept, see "Reload" step 3), its chunks read from `world/DIM-1/region`, so the
  blocks, the pig and the chest are there after the reload.
- `tick/gate2-session` (Tier S, seed 123 terrain, difficulty 1, natural spawning off, 411
  ticks): PLAN_v4 Gate 2's headless session. Walk (60 ticks downhill from the grass at
  (−25.5, 67, −57.5)), dig the grass in front with a stone shovel and walk into the hole to
  pick up the dirt, place dirt (`use_item`), craft in the player's 2×2 grid by `slot_click`
  (logs → planks, planks → sticks, planks → crafting table) plus three `craft` ops, place the
  crafting table, fight a zombie (spawned at t = 0 out of the player's sight, so its id does not depend on the terrain and the flipped-seed check can run the scenario, then moved 6 blocks from the player by `set_pos`; three sword hits), eat bread, set the
  time to night, place the bed (`use_item` with the bed item: ItemBed.onItemUse; no
  `set_block` fallback), `sleep` (`OK`; `World.tick` skips to the next morning after 100
  ticks and wakes the player), `save` (Tier S lines for chunks (−2, −4) and (−2, −3), which
  hold every edit; the rest of the 3×3 around the player is unedited generated terrain,
  compared by the `chunk_digest` dumps every 20 ticks instead of ~165 kB `chunk_nbt` lines),
  `reload`, 100 more ticks, a final `save` (same chunks). The
  scenario's `comment` lists the steps and ticks. It passes on all 3,237 lines since WP-2.H
  transcribed `EntityItem.onCollideWithPlayer` (it was ignored at Gate 2, first diff at line
  745, t = 78).

Vanilla behaviour these fixtures pin down:

- **Still water placed by `set_block` stays still**: `notifyBlockChange` notifies the six
  neighbours, never the block itself, and `BlockStationary` only becomes flowing water from
  `onNeighborBlockChange` (BlockStationary.java:15-21); its `onBlockAdded` only checks for
  hardening (BlockFluid.java:239-241). Buckets place flowing water, id 8 (ItemBucket.java:87).
- **Generated sand falls on the first ticks**: generation and population leave sand above
  air with ticks scheduled by `onBlockAdded`; they fire in the first `TickUpdates` (population
  itself runs with `BlockSand.fallInstantly = true`, ChunkProviderGenerate.java:311, :602).
- **Light updates run newest first, at most 499 per `updatingLighting` call**:
  `lightingToUpdate` is a list popped from the end (World.java:1674); the budget is
  `var1 = 500`, decremented before each update and checked `<= 0` (World.java:1665-1672), so
  the 500th pending update waits for the next call and the call returns `true`. A new update
  is first offered to the five newest queued updates of the same type, which may absorb it by
  growing their box (`MetadataChunkBlock.func_866_a`, World.java:1702-1713), and it is
  dropped when its box's centre column is not loaded (World.java:1696-1699). Light therefore
  spreads over several ticks after large edits (`light/skylight-column` shows the queue
  outliving one tick; `light/blocklight-glowstone` pins the exact budget, failing at 498 and
  at 500 updates per call).

## Soak (WP-4.2)

`tests/golden/soak/<name>`: long-run `tick` scenarios (PLAN_v4 §5 Phase 4 row 4.2), tens of
thousands of ticks each, with periodic world digests compared against the oracle, as a
repeatable check that the port does not drift from the Java over long runs. They are
ordinary `kind: "tick"` scenarios: only the ops and dumps documented above, no soak-only
harness code.

| Fixture | World | Ticks | Sites (X, Z), from tick | Also | Lines / size | Oracle | Rust |
|---|---|---|---|---|---|---|---|
| `soak/seed-123` | seed 123, generated, `"save_format": "mcregion"` | 72,000 | (8, 8) 0; (408, 8) 18,000; (−92, −442) 36,000; (8, 8) 54,000 | rain from 24,000; `reload` at 55,000 (player id 0 → 7848) | 6,156 / 3.1 MB | 31 s | 136–155 s |
| `soak/seed-8675309` | seed 8675309, generated, no save | 48,000 | (−40, 24) 0; (−40, 524) 12,000; (−640, 24) 24,000; (−40, 24) 36,000 | rain from 6,000 | 4,474 / 2.4 MB | 19–20 s | 42–81 s alone, 99–115 s in the set |
| `soak/seed--7612233452384654969` | seed −7612233452384654969, generated, `mcregion` | 48,000 | (8, 8) 0; (20008, −15000) 12,000; (−3000, 2504) 24,000; (8, 8) 36,000 | rain from 30,000; `reload` at 37,000 (player 0 → 8166) | 4,196 / 2.1 MB | 28 s | 109–124 s |
| `soak/testworld` | the owner's testWorld, `"world": "load"` (as `tick/testworld-6000`) | 48,000 | the spawn sand (21.5, 65, −63.5) 0 (garden site (45, −64) built at 0 beside it); (296, −296) 16,000; the spawn sand 32,000 | player id 9; rain from 20,000 | 3,405 / 1.8 MB | 24 s | 105–119 s |
| `soak/nether-8675309` | seed 8675309, nether, generated, no save | 36,000 | (8, 8) 0; (208, −192) 12,000; (8, 8) 24,000 | no weather in the nether; a new pig and chicken at 24,000; a minecart for the whole run | 1,560 / 0.7 MB | 21 s | 94–104 s |

Runtimes were measured on the 16-core development machine (OpenJDK 25; Rust `test` profile,
i.e. `dev`: opt-level 1 for the workspace crates, 3 for dependencies) while other builds ran,
so they vary by up to 2x between runs. Oracle: one JVM per fixture, start-up included
(~2,300 ticks/s for `seed-123`); all five in one `SOAK=only regen.sh` run take about 2 min.
Rust: the five tests run in parallel, so the set takes as long as `seed-123`, 2.5 min; the
ranges are over two runs of the set (each fixture runs at 350–530 ticks/s while the set runs in
parallel). `seed-8675309` alone, four runs each: 67–81 s in the test profile (about 700 ticks/s;
one run took 42 s) and 39–41 s with opt-level 3 for the workspace crates too
(`CARGO_PROFILE_DEV_OPT_LEVEL=3`, about 1,180 ticks/s; one run took 68 s), against 19–20 s
for the oracle (about 2,400 ticks/s including the JVM start-up): the port runs at roughly a
third (opt-level 1) to a half (opt-level 3) of the oracle's speed, a data point for WP-4.3.

**What a site is.** Each overworld site is a 15 × 9 stone platform at y = 100
(x = X−7…X+7, z = Z−4…Z+4; the 15 × 12 × 9 box above it is first filled with air), built with
`fill`/`set_block` ops in the tick the player arrives, so the site does not depend on the
terrain and the scenarios run under `check-sensitivity.sh`'s flipped seed. On it: the player
at (X−5.5, 101, Z+0.5); a dirt block beside it dug with a stone shovel from t+2 (the drop is
picked up), dirt placed back with `use_item` at t+20 and a torch at t+21; birch, spruce and
oak saplings on dirt (random-tick growth into `WorldGenForest`, `WorldGenTaiga2`,
`WorldGenTrees` or `WorldGenBigTree`); a farm row (crops on two farmland blocks around a
still-water block, a cactus on sand and reeds on dirt beside the water); two leaves blocks with
the check-decay bit and no log in range; sand and gravel in the air; a fenced 2 × 2 pen with
a pig and a chicken; a cobblestone item out of the player's reach (it despawns at age 6000).
Outside the platform, three one-block pillars: flowing water at (X+11, 101, Z) and flowing
lava at (X, 101, Z−9) pour down to the terrain (long fluid cascades; lava fires on the
ground) and an eternal fire on netherrack at (X+11, 101, Z−4) burns a plank beside it. A
nether site is a 15 × 9 room at floor y = 64 inside a cobblestone shell (so the terrain's
lava cannot reach it) with glowstone in the ceiling, two mushrooms (spreading on random ticks),
an eternal fire, a lava source in a basin with a plank above it, soul sand, falling gravel, a
fenced pen with a pig and a chicken, an item, the same dig (netherrack, stone pickaxe) and
placing; the first nether site also has a plain minecart on a rail, which never despawns, and
a new pig and chicken go into its pen when the player returns (the first pair despawned when
it left), so the nether run has living and non-living entities updating to the end. The player moves to the
next site with `set_pos` (y = feet + `(double) 1.62F`, the player's `yOffset`, because
`set_pos` is `Entity.setPosition` while the `player` op adds it) and `set_motion` 0; a
revisited site is not rebuilt. At a new site the player's 19 × 19 chunks are generated and
populated; nothing unloads (b1.7.3 single player never drops chunks); animals left more than
128 blocks behind despawn (`EntityLiving.despawnEntity`). Each `scenario.json`'s `comment`
lists its sites and ops.

**Reproducible settings.** Difficulty 0 with natural spawning on (`mob_spawning` default):
animals and squid spawn, hostile spawning stays off (census §3: hostile spawn checks draw
from an unseeded `Entity.rand`). Every overworld scenario sets `thunder_time` to 100000000
with `set_weather` at t = 0, so no thunderstorm starts: lightning bolts draw their lifetime
from a never-seeded `Entity.rand` (EntityLightningBolt.java:14-15, census §3) and a bolt near
an entity would make the oracle irreproducible. Rain is natural (a `rain_time` makes the
first rain start inside the run). `reload` comes after the last `set_pos`: the reloaded
player's id (from the oracle's `reload` line) depends on how many entities spawned, so under
the flipped seed it differs, and the every-500 `entity_state` of that id then gives
`entity_absent` instead of failing an op.

**Dumps.** `rng_state` every 100 ticks (the first-divergence signal: random block ticks,
spawning and `randomDisplayUpdates` all draw `World.rand`); the player's `entity` line every
500; `world_time`, `weather`, `scheduled_ticks`, `player_state`, `inventory`, `entity_state`
`"all"` and the `chunk_digest` of the 3 × 3 chunks around every site every 1,000; at the last
tick also the rest of each site's 5 × 5 chunks, a `block_box` over each site (for the
testWorld spawn: y = 58…71, with the chest at (9, 64, −61)) and `scheduled_ticks` with
`full`. Chunk digests cover these fixed chunks, not every loaded chunk (no dump lists the
loaded set); a difference elsewhere in the player's 19 × 19 shows up in `rng_state` or the
entity lines. No `save` op: `crates/mc-sim/tests/golden_save.rs` requires every fixture with
one to be registered there, so the two `mcregion` scenarios (and `testworld`'s copy) exercise
saving through the autosave every 40 ticks and the `reload` op instead.

**Selected by `SOAK`.** `tools/oracle/env.sh`'s `golden_ids` picks the scenarios every
oracle script runs: `SOAK` unset or `0` (the default) leaves `tests/golden/soak` out, so the
default runs are unchanged; `SOAK=only` selects the soak set alone, `SOAK=1` everything.

```sh
SOAK=only sh tools/oracle/regen.sh               # regenerate tests/golden/soak (~2 min)
SOAK=only sh tools/oracle/check-reproducible.sh  # two oracle runs byte-identical (~4 min)
SOAK=only sh tools/oracle/check-sensitivity.sh   # every soak fixture changes under seed ^ 1 (~2 min)
cargo test -p mc-golden --test golden_soak -- --ignored --nocapture   # the Rust side (~2.5 min)
```

`crates/mc-golden/tests/golden_soak.rs` has one `#[ignore]`d test per fixture (`--nocapture`
prints its ticks per second) and a non-ignored check that every `tests/golden/soak/*`
directory has one. The ordinary `cargo test` therefore runs no soak scenario, but it still
reads the soak `expected.jsonl` files where a test reads every fixture
(`fixtures_parse.rs`, `constructors_match_oracle.rs`), which adds about 0.5 s to
`cargo test -p mc-golden` (0.9 s → 1.4 s here). The CI workflow's `soak` job runs the
Rust side on a manual run with the `soak` input ticked. A failing soak test names the first
differing line; the Rust trace is in `target/golden/soak/<name>/actual.jsonl`, and the
`rng_state` lines bound the first divergence to 100 ticks, from where a short scenario cut at
that tick reproduces it.

Status (WP-4.2): all five pass on every line.

