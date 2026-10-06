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

## Tier A scenarios (WP-0.2, WP-0.3, WP-1.1)

Common fields: `"name"` (must equal the scenario directory name), `"tier": "A"`,
`"kind"`. Kind-specific fields below. Every trace line, including the last, ends
in `"\n"`. Scenarios run in sorted path order in one JVM, but no scenario's output
depends on another: the oracle reseeds its `Math.random()` replacement stream
(`HarnessMath`, see `tools/oracle/README.md`) from the scenario `seed` (0 when there
is none) at the start of every scenario. A scenario without a `seed` is allowed only
for the kinds whose output is a pure function of the scenario file: `sine_table`,
`nbt_write`, `nbt_read`, `java_hash_order`, `block_table`, `item_table`
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
| `world` | `generated` (default; the dimension's terrain generator) \| `empty` (every chunk all air) |
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
| `player` | `x,y,z` (feet), `yaw?,pitch?` | `new HarnessPlayer(world)`, seeded, `setLocationAndAngles`, `entityJoinedWorld`. At most one | `id`, `joined` |
| `spawn` | `kind`, `pos:[x,y,z]` (bounding-box bottom), `yaw?,pitch?`, `id?`; for `kind: "Item"`: `item:{id,count,damage?}`, `pickup_delay?` (10) | `new Entity<kind>(world)` with the class from EntityList's name table (EntityList.java:82-107: `Arrow`, `Snowball`, `Item`, `Painting`, `Mob`, `Monster`, `Creeper`, `Skeleton`, `Spider`, `Giant`, `Zombie`, `Slime`, `Ghast`, `PigZombie`, `Pig`, `Sheep`, `Cow`, `Chicken`, `Squid`, `Wolf`, `PrimedTnt`, `FallingSand`, `Minecart`, `Boat`); `rand.setSeed(mix(seed, id))` (Slime: size and jump delay redrawn); `setLocationAndAngles(x, y, z, yaw, pitch)` (Entity.java:669-676: `posY = y + yOffset`; yaw/pitch default to the constructor's, a `Math.random()` yaw for living entities); `entityJoinedWorld`. `Item` instead uses `new EntityItem(world, x, y, z, stack)` (EntityItem.java:11-21: `Math.random()` yaw and horizontal motion, `motionY` 0.2, position via `setPosition`) and `delayBeforeCanPickup = pickup_delay`, as block drops do (Block.java:355-364). `id`, if given, must equal the entity id it gets | `id`, `kind` (class simple name), `joined` |
| `set_motion` | `id`, `motion:[x,y,z]` | assigns `motionX/Y/Z` | `id` |
| `set_pos` | `id`, `pos:[x,y,z]` | `Entity.setPosition` (Entity.java:163-170; `prevPos*` untouched) | `id` |
| `set_rotation` | `id`, `yaw`, `pitch` | assigns `rotationYaw`/`rotationPitch` (no `% 360`) | `id` |
| `use_item` | `item_id?`, `count?` (1), `meta?` (0), `x,y,z,side` | needs a player. The stack (if `item_id` is given) replaces hotbar slot `currentItem`; then the right-click-on-block branch of `Minecraft.clickMouse(1)` (Minecraft.java:819-850) with the op's hit instead of a ray trace: `PlayerController.sendPlaceBlock` (PlayerController.java:75-82: the block's `blockActivated`, else `ItemStack.useItem` → `Item.onItemUse`); on success `swingItem`; a stack left at size 0 is removed; if nothing was placed, `sendUseItem` (`useItemRightClick`, PlayerController.java:47-61) | `placed`, `used`, `stack` (`{id,count,damage}` or `null`, the slot afterwards) |
| `attack` | `target_id`, `attacker_id?`, `damage` (unless the attacker is the player) | attacker = the player: `swingItem` then `attackTargetEntityWithCurrentItem` (Minecraft.java:802-814, PlayerController.java:92-94). Otherwise `target.attackEntityFrom(attacker or null, damage)` (Entity.java:746, EntityLiving.java:307) | `target_id`, `via` (`player` \| `direct`), `result` (direct only) |
| `explode` | `x,y,z` (f64: integer or bit pattern), `size` (f32), `flaming?` (false) | `World.createExplosion(null, x, y, z, size)` (World.java:1518-1520: `newExplosion(…, false)`, Explosion.java:29-164). With `flaming`: the body of `World.newExplosion(null, x, y, z, size, true)` (World.java:1522-1528) with the explosion's `ExplosionRNG` (a clock-seeded `new Random()`, Explosion.java:11, census §2) set to `mix(seed, -6)` before `doExplosionA`; only the fire roll draws it (Explosion.java:118). No exploder: no entity is excluded, the attacker is null. Not part of the implicit preload (WP-1.8) | `destroyed` (size of `destroyedBlockPositions`) |
| `save` | `chunks?` (list of `{x,z}`) | Tier S, below; needs `"save_format": "mcregion"` | — then the Tier S lines |

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
  from items (`onBlockPlaced`, `onBlockPlacedBy`). The port's test is ignored until
  `Item.onItemUse` lands (WP-2.6).

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
- `tick/door-iron-powered` (redstone torch: doors and trapdoors opened by power, and the
  trapdoor's power branch running on its emptied cell, BlockTrapDoor.java:115-123),
  `tick/door-upper-powered` (lever: the upper half forwarding a power notification to the
  lower half, BlockDoor.java:141-143) and `tick/portal-ignite` (`BlockFire.onBlockAdded` →
  `tryToCreatePortal`) need the redstone and fire tasks, both in.
- `tick/bed-nether` needs `Explosion` (WP-1.8, in). Ignored on the Rust side until its
  dependencies land: `tick/block-item-placement` (`ItemBlock` and the other placing items,
  WP-2.6).

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
  draws per tick); `tick/mob-spawner-spawn` continues into the spawn attempts (needs the
  `EntityPig` constructor in a port).
- `tick/sign-pops-off`: wall signs and a sign post losing their support (`BlockSign.onNeighborBlockChange`
  drops an `Item.sign`), a wall sign with metadata 0 dropping at the first neighbour change.
- `save/tile-entities` (Tier S, player): one tile entity of each class in chunk (0, 0), the note
  block clicked twice, then `save`: the `chunk_nbt` `TileEntities` list and the `tile_entity` lines
  pin every `writeToNBT` shape and the `chunkTileEntityMap` order.

Not pinned by any fixture: the lightning strike path of `updateBlocksAndPlayCaveSounds`
past its `nextInt(100000)` roll (World.java:1920-1930; on the weather-snow platform a strike
would need the one-in-100000 roll on a platform column of a rainy biome, and
`EntityLightningBolt` is WP-2.4), and so `canBlockBeRainedOn`'s biome test
(World.java:2449-2460). A roll that does succeed on an empty column stops at
`canBlockSeeTheSky` (y = -1), before the biome test.

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
