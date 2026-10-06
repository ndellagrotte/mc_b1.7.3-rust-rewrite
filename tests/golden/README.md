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
`nbt_write`, `nbt_read`, `java_hash_order`, `block_table`
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
`Block.dispenser`'s = `mix(seed, -5)`. The `Math.random()` stream is
`JavaRandom::new(seed)`, drawn only at census §1's S and D sites.

**Tick loop.** Before tick 0 the implicit preload runs (below) and emits
`{"t":0,"e":"preload","implicit":true,"loaded":N}`. Then for each `t`:

1. `AxisAlignedBB.clearBoundingBoxPool()`, `Vec3D.initialize()` (frame start, Minecraft.java:519-520).
2. The ops with this `t`, in file order.
3. If there is a player: `if (++joinPlayerCounter == 30) { joinPlayerCounter = 0; world.joinEntityInSurroundings(player); }` (Minecraft.java:1134-1139).
4. `world.difficultySetting = difficulty`; `if (world.field_27172_i > 0) --world.field_27172_i` (Minecraft.java:1142, :1155-1157).
5. `world.updateEntities()` (World.java:1178-1276).
6. `world.setAllowedMobSpawns(difficulty > 0, true)`; `world.tick()` (World.java:1745-1780).
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
| `preload` | `cx,cz,radius` | preload square (above) | `loaded` |
| `player` | `x,y,z` (feet), `yaw?,pitch?` | `new HarnessPlayer(world)`, seeded, `setLocationAndAngles`, `entityJoinedWorld`. At most one | `id`, `joined` |
| `spawn` | `kind`, `pos:[x,y,z]` (bounding-box bottom), `yaw?,pitch?`, `id?`; for `kind: "Item"`: `item:{id,count,damage?}`, `pickup_delay?` (10) | `new Entity<kind>(world)` with the class from EntityList's name table (EntityList.java:82-107: `Arrow`, `Snowball`, `Item`, `Painting`, `Mob`, `Monster`, `Creeper`, `Skeleton`, `Spider`, `Giant`, `Zombie`, `Slime`, `Ghast`, `PigZombie`, `Pig`, `Sheep`, `Cow`, `Chicken`, `Squid`, `Wolf`, `PrimedTnt`, `FallingSand`, `Minecart`, `Boat`); `rand.setSeed(mix(seed, id))` (Slime: size and jump delay redrawn); `setLocationAndAngles(x, y, z, yaw, pitch)` (Entity.java:669-676: `posY = y + yOffset`; yaw/pitch default to the constructor's, a `Math.random()` yaw for living entities); `entityJoinedWorld`. `Item` instead uses `new EntityItem(world, x, y, z, stack)` (EntityItem.java:11-21: `Math.random()` yaw and horizontal motion, `motionY` 0.2, position via `setPosition`) and `delayBeforeCanPickup = pickup_delay`, as block drops do (Block.java:355-364). `id`, if given, must equal the entity id it gets | `id`, `kind` (class simple name), `joined` |
| `set_motion` | `id`, `motion:[x,y,z]` | assigns `motionX/Y/Z` | `id` |
| `set_pos` | `id`, `pos:[x,y,z]` | `Entity.setPosition` (Entity.java:163-170; `prevPos*` untouched) | `id` |
| `set_rotation` | `id`, `yaw`, `pitch` | assigns `rotationYaw`/`rotationPitch` (no `% 360`) | `id` |
| `use_item` | `item_id?`, `count?` (1), `meta?` (0), `x,y,z,side` | needs a player. The stack (if `item_id` is given) replaces hotbar slot `currentItem`; then the right-click-on-block branch of `Minecraft.clickMouse(1)` (Minecraft.java:819-850) with the op's hit instead of a ray trace: `PlayerController.sendPlaceBlock` (PlayerController.java:75-82: the block's `blockActivated`, else `ItemStack.useItem` → `Item.onItemUse`); on success `swingItem`; a stack left at size 0 is removed; if nothing was placed, `sendUseItem` (`useItemRightClick`, PlayerController.java:47-61) | `placed`, `used`, `stack` (`{id,count,damage}` or `null`, the slot afterwards) |
| `attack` | `target_id`, `attacker_id?`, `damage` (unless the attacker is the player) | attacker = the player: `swingItem` then `attackTargetEntityWithCurrentItem` (Minecraft.java:802-814, PlayerController.java:92-94). Otherwise `target.attackEntityFrom(attacker or null, damage)` (Entity.java:746, EntityLiving.java:307) | `target_id`, `via` (`player` \| `direct`), `result` (direct only) |
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
`save/roundtrip-seed-123`, `save/testworld`.

Empty world (`"world": "empty"`), stone platform at y = 60 over −8…8 with a one-high rim at
y = 61, built by `fill` at t = 0: `tick/flat-water-flow` (still water, id 9: it never flows,
see below), `tick/flat-flowing-water` (id 8), `tick/flat-sand-falls` (an
`EntityFallingSand` appears and lands), `tick/flat-torch-light` (block light),
`entity/flat-item-drop`, `entity/flat-pig-falls`. They exist so a port can verify one block
and one entity before it has terrain generation.

Vanilla behaviour these fixtures pin down:

- **Still water placed by `set_block` stays still**: `notifyBlockChange` notifies the six
  neighbours, never the block itself, and `BlockStationary` only becomes flowing water from
  `onNeighborBlockChange` (BlockStationary.java:15-21); its `onBlockAdded` only checks for
  hardening (BlockFluid.java:239-241). Buckets place flowing water, id 8 (ItemBucket.java:87).
- **Generated sand falls on the first ticks**: generation and population leave sand above
  air with ticks scheduled by `onBlockAdded`; they fire in the first `TickUpdates` (population
  itself runs with `BlockSand.fallInstantly = true`, ChunkProviderGenerate.java:311, :602).
