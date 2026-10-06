package harness;

import static harness.Main.Scenario.flag;
import static harness.Main.Scenario.list;
import static harness.Main.Scenario.num;
import static harness.Main.Scenario.seed;
import static harness.Main.Scenario.str;

import java.lang.reflect.Field;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.Random;
import net.minecraft.src.BiomeGenBase;
import net.minecraft.src.Chunk;
import net.minecraft.src.IChunkProvider;
import net.minecraft.src.SaveHandlerMP;
import net.minecraft.src.World;
import net.minecraft.src.WorldChunkManager;
import net.minecraft.src.WorldProvider;
import net.minecraft.src.WorldProviderHell;
import net.minecraft.src.WorldProviderSurface;

/** Tier A scenario kinds, exactly as specified in tests/golden/README.md. */
final class TierA {
    private TierA() {}

    /** {@code dir} is the scenario directory (input sidecars, e.g. {@code nbt_read}'s file). */
    static Trace run(Map<String, Object> sc, Path dir) throws Exception {
        String kind = str(sc, "kind");
        // Every vanilla Math.random() call draws from HarnessMath; reseed it per
        // scenario so a scenario's output never depends on the ones before it.
        HarnessMath.seed(sc.containsKey("seed") ? seed(sc) : 0L);
        switch (kind) {
            case "item_table": return ItemTable.run(sc); // WP-1.2
            case "java_random": return javaRandom(sc);
            case "sine_table": return sineTable();
            case "math_sweep": return mathSweep(sc);
            case "terrain": return terrain(sc);
            case "nbt_write": return Nbt.write(sc);
            case "nbt_read": return Nbt.read(sc, dir);
            case "java_hash_order": return HashOrder.run(sc);
            case "block_table": return BlockTable.run(sc);
            default: throw new IllegalArgumentException("unknown kind " + kind);
        }
    }

    // ---- java_random ---------------------------------------------------------

    @SuppressWarnings("unchecked")
    private static Trace javaRandom(Map<String, Object> sc) {
        Trace t = new Trace();
        Random r = new Random(seed(sc));
        long i = 0;
        for (Object o : list(sc, "draws")) {
            Map<String, Object> d = (Map<String, Object>) o;
            String call = str(d, "call");
            boolean bounded = d.containsKey("bound");
            long n = num(d, "n");
            for (long k = 0; k < n; k++) {
                Json.Line l = new Json.Line().put("t", 0).put("e", "rng").put("i", i++).put("call", call);
                switch (call) {
                    case "nextInt":
                        if (bounded) {
                            int bound = Math.toIntExact(num(d, "bound"));
                            l.put("bound", bound).put("v", r.nextInt(bound));
                        } else {
                            l.put("v", r.nextInt());
                        }
                        break;
                    case "nextLong": l.put("v", r.nextLong()); break;
                    case "nextFloat": l.f32("v", r.nextFloat()); break;
                    case "nextDouble": l.f64("v", r.nextDouble()); break;
                    case "nextGaussian": l.f64("v", r.nextGaussian()); break;
                    case "nextBoolean": l.put("v", r.nextBoolean()); break;
                    default: throw new IllegalArgumentException("unknown draw call " + call);
                }
                if (bounded && !call.equals("nextInt")) throw new IllegalArgumentException("bound only applies to nextInt");
                t.add(l);
            }
        }
        return t;
    }

    // ---- sine_table ----------------------------------------------------------

    private static Trace sineTable() throws Exception {
        // src: MathHelper.java:4 (private static float[] SIN_TABLE), filled in the static
        // initialiser at MathHelper.java:56-60. Class.forName(..., true, ...) runs it.
        Class<?> mh = Class.forName("net.minecraft.src.MathHelper", true, TierA.class.getClassLoader());
        Field f = mh.getDeclaredField("SIN_TABLE");
        f.setAccessible(true);
        float[] table = (float[]) f.get(null);
        ByteBuffer buf = ByteBuffer.allocate(table.length * 4).order(ByteOrder.LITTLE_ENDIAN);
        for (float v : table) buf.putInt(Float.floatToRawIntBits(v));
        byte[] bytes = buf.array();
        Trace t = new Trace();
        String file = "sine_table.f32le.bin";
        t.add(new Json.Line().put("t", 0).put("e", "sine_table").put("len", table.length)
                .put("blake3", Blake3.hex(bytes)).put("file", file));
        t.sidecars.put(file, bytes);
        return t;
    }

    // ---- math_sweep ----------------------------------------------------------

    @SuppressWarnings("unchecked")
    private static Trace mathSweep(Map<String, Object> sc) {
        Trace t = new Trace();
        Random r = new Random(seed(sc));
        long count = num(sc, "count");
        boolean strict = flag(sc, "strict");
        Map<String, Object> edges = (Map<String, Object>) sc.get("edge_cases");
        if (edges == null) throw new IllegalArgumentException("missing \"edge_cases\"");
        for (Object fo : list(sc, "functions")) {
            String fn = (String) fo;
            boolean two = TWO_ARG.contains(fn);
            long i = 0;
            for (Object e : list(edges, fn)) {
                double x, y = 0;
                if (two) {
                    List<Object> pair = (List<Object>) e;
                    if (pair.size() != 2) throw new IllegalArgumentException(fn + " edge case must be [x, y]");
                    x = Hex.parseF64((String) pair.get(0));
                    y = Hex.parseF64((String) pair.get(1));
                } else {
                    x = Hex.parseF64((String) e);
                }
                t.add(mathLine(fn, i++, two, x, y, strict));
            }
            for (long k = 0; k < count; k++) {
                double x = r.nextDouble() * 2000.0D - 1000.0D;
                double y = two ? r.nextDouble() * 2000.0D - 1000.0D : 0;
                t.add(mathLine(fn, i++, two, x, y, strict));
            }
        }
        return t;
    }

    private static final List<String> TWO_ARG = List.of("atan2", "pow");

    /**
     * java.lang.Math (what vanilla calls), or java.lang.StrictMath when the scenario sets
     * {@code "strict": true}. Arguments in (x, y) order.
     */
    private static Json.Line mathLine(String fn, long i, boolean two, double x, double y, boolean strict) {
        double v;
        switch (fn) {
            case "sin": v = strict ? StrictMath.sin(x) : Math.sin(x); break;
            case "cos": v = strict ? StrictMath.cos(x) : Math.cos(x); break;
            case "tan": v = strict ? StrictMath.tan(x) : Math.tan(x); break;
            case "atan": v = strict ? StrictMath.atan(x) : Math.atan(x); break;
            case "sqrt": v = strict ? StrictMath.sqrt(x) : Math.sqrt(x); break;
            case "exp": v = strict ? StrictMath.exp(x) : Math.exp(x); break;
            case "log": v = strict ? StrictMath.log(x) : Math.log(x); break;
            case "log10": v = strict ? StrictMath.log10(x) : Math.log10(x); break;
            case "atan2": v = strict ? StrictMath.atan2(x, y) : Math.atan2(x, y); break;
            case "pow": v = strict ? StrictMath.pow(x, y) : Math.pow(x, y); break;
            default: throw new IllegalArgumentException("unknown math function " + fn);
        }
        Json.Line l = new Json.Line().put("t", 0).put("e", "math").put("fn", fn).put("i", i).f64("x", x);
        if (two) l.f64("y", y);
        return l.f64("v", v);
    }

    // ---- terrain -------------------------------------------------------------

    @SuppressWarnings("unchecked")
    private static Trace terrain(Map<String, Object> sc) throws Exception {
        long seed = seed(sc);
        String dim = str(sc, "dimension");
        boolean overworld;
        WorldProvider provider;
        switch (dim) {
            case "overworld": overworld = true; provider = new WorldProviderSurface(); break;
            case "nether": overworld = false; provider = new WorldProviderHell(); break;
            default: throw new IllegalArgumentException("unknown dimension " + dim);
        }
        // src: World.java:62. SaveHandlerMP: no disk, no level.dat, no chunk loader.
        World world = new World(new SaveHandlerMP(), "harness", provider, seed);
        HarnessWorld.makeDeterministic(world, seed);
        IChunkProvider chunks = world.getIChunkProvider();

        Trace t = new Trace();
        for (Object o : list(sc, "chunks")) {
            Map<String, Object> c = (Map<String, Object>) o;
            int x = Math.toIntExact(num(c, "x"));
            int z = Math.toIntExact(num(c, "z"));
            boolean full = flag(c, "full");

            // 1. generated: the chunk as first returned by the world's ChunkProvider.
            for (int dz = -1; dz <= 1; dz++) {
                for (int dx = -1; dx <= 1; dx++) {
                    if (chunks.chunkExists(x + dx, z + dz)) {
                        System.err.println("harness: warning: chunk (" + (x + dx) + "," + (z + dz)
                                + ") already loaded before " + x + "," + z
                                + "; the \"generated\" stage is not pure generator output");
                    }
                }
            }
            Chunk chunk = world.getChunkFromChunkCoords(x, z);
            chunkDigest(t, 0, chunk, x, z, "generated", full);

            // 2. populated: load the 3x3 neighbourhood, dz outer, dx inner.
            for (int dz = -1; dz <= 1; dz++) {
                for (int dx = -1; dx <= 1; dx++) {
                    world.getChunkFromChunkCoords(x + dx, z + dz);
                }
            }
            if (!chunk.isTerrainPopulated) {
                throw new IllegalStateException("chunk " + x + "," + z + " not populated after loading its neighbours");
            }
            chunkDigest(t, 0, chunk, x, z, "populated", full);

            // 3. biome (overworld only). A fresh WorldChunkManager gives the same arrays as
            // the world's own (the noise is a pure function of seed and coordinates) without
            // touching the world's cached temperature/humidity arrays.
            if (overworld) {
                WorldChunkManager wcm = new WorldChunkManager(world);
                BiomeGenBase[] biomes = wcm.loadBlockGeneratorData(null, x * 16, z * 16, 16, 16);
                StringBuilder names = new StringBuilder();
                for (int i = 0; i < 256; i++) {
                    if (i > 0) names.append('\n');
                    names.append(biomes[i].biomeName);
                }
                t.add(new Json.Line().put("t", 0).put("e", "biome").put("x", x).put("z", z)
                        .put("biomes", Blake3.hex(names.toString().getBytes(StandardCharsets.UTF_8)))
                        .put("temperature", Blake3.hex(f64le(wcm.temperature, 256)))
                        .put("humidity", Blake3.hex(f64le(wcm.humidity, 256))));
            }
        }
        return t;
    }

    /** {@code chunk_digest} (+ {@code chunk_full}) line at tick {@code tick}; shared with Tier B/C/S. */
    static void chunkDigest(Trace t, long tick, Chunk c, int x, int z, String stage, boolean full) {
        byte[][] arrays = {c.blocks, c.data.data, c.skylightMap.data, c.blocklightMap.data, c.heightMap};
        String[] keys = {"blocks", "data", "skylight", "blocklight", "heightmap"};
        int[] lengths = {32768, 16384, 16384, 16384, 256};
        Json.Line digest = new Json.Line().put("t", tick).put("e", "chunk_digest").put("x", x).put("z", z).put("stage", stage);
        Json.Line dump = new Json.Line().put("t", tick).put("e", "chunk_full").put("x", x).put("z", z).put("stage", stage);
        for (int i = 0; i < arrays.length; i++) {
            if (arrays[i].length != lengths[i]) {
                throw new IllegalStateException(keys[i] + " has length " + arrays[i].length + ", expected " + lengths[i]);
            }
            digest.put(keys[i], Blake3.hex(arrays[i]));
            dump.put(keys[i], Hex.of(arrays[i]));
        }
        t.add(digest);
        if (full) t.add(dump);
    }

    private static byte[] f64le(double[] values, int n) {
        ByteBuffer buf = ByteBuffer.allocate(n * 8).order(ByteOrder.LITTLE_ENDIAN);
        for (int i = 0; i < n; i++) buf.putLong(Double.doubleToRawLongBits(values[i]));
        return buf.array();
    }
}
