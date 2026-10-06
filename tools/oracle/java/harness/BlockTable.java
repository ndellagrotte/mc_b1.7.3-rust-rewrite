package harness;

import java.io.File;
import java.lang.reflect.Array;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collection;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.TreeMap;

/**
 * Tier A kind {@code block_table} (WP-1.1; tests/golden/README.md "{@code kind: "block_table"}"):
 * every {@code Block.blocksList} entry, the per-id static arrays, every {@code Material} and
 * {@code MapColor}, as they are right after {@code Block}'s static initialiser (Block.java:592-706).
 *
 * <p>The vanilla classes are loaded into a fresh class loader so the dump is the pristine
 * post-initialiser state whatever ran earlier in the JVM (tick scenarios mutate block singletons:
 * {@code BlockFlowing}'s scratch fields, bounds, ...). Everything vanilla is reached by reflection.
 * {@code BlockLeavesBase.graphicsLevel} keeps its static default (false; no renderer runs).
 */
final class BlockTable {
    private BlockTable() {}

    private static final String SRC = "net.minecraft.src.";

    /** Classes of the fresh loader. */
    private static Class<?> blockCls, materialCls, stepSoundCls, mapColorCls;
    /** Material singleton → its static field name in Material.java. */
    private static final Map<Object, String> MATERIAL_NAMES = new IdentityHashMap<>();

    static Trace run(Map<String, Object> sc) throws Exception {
        if (sc.containsKey("seed")) throw new IllegalArgumentException("block_table takes no seed");
        try (URLClassLoader fresh = freshLoader()) {
            blockCls = Class.forName(SRC + "Block", true, fresh);
            materialCls = Class.forName(SRC + "Material", true, fresh);
            stepSoundCls = Class.forName(SRC + "StepSound", true, fresh);
            mapColorCls = Class.forName(SRC + "MapColor", true, fresh);
            MATERIAL_NAMES.clear();
            for (Field f : materialCls.getDeclaredFields()) {
                if (Modifier.isStatic(f.getModifiers()) && f.getType() == materialCls) {
                    MATERIAL_NAMES.put(f.get(null), f.getName());
                }
            }
            Trace t = new Trace();
            Object[] blocksList = (Object[]) blockCls.getField("blocksList").get(null);
            for (int id = 0; id < 256; id++) t.add(blockLine(id, blocksList[id]));
            t.add(staticsLine(blocksList));
            for (Map.Entry<String, Object> m : sortedMaterials().entrySet()) t.add(materialLine(m.getKey(), m.getValue()));
            Object[] colors = (Object[]) mapColorCls.getField("mapColorArray").get(null);
            for (int i = 0; i < colors.length; i++) t.add(mapColorLine(i, colors[i]));
            return t;
        }
    }

    /** A loader over the harness's own class path whose parent cannot see the vanilla classes. */
    private static URLClassLoader freshLoader() throws Exception {
        String[] parts = System.getProperty("java.class.path").split(File.pathSeparator);
        URL[] urls = new URL[parts.length];
        for (int i = 0; i < parts.length; i++) urls[i] = Paths.get(parts[i]).toAbsolutePath().toUri().toURL();
        return new URLClassLoader(urls, ClassLoader.getPlatformClassLoader());
    }

    // ---- lines -------------------------------------------------------------------

    private static Json.Line blockLine(int id, Object b) throws Exception {
        Json.Line l = new Json.Line().put("t", 0).put("e", "block_def").put("id", id);
        if (b == null) return l.put("present", false);
        l.put("present", true).put("class", b.getClass().getSimpleName());
        l.raw("fields", fields(b, blockCls));
        l.raw("isOpaqueCube", call(b, "isOpaqueCube"));
        l.raw("renderAsNormalBlock", call(b, "renderAsNormalBlock"));
        l.raw("getRenderType", call(b, "getRenderType"));
        l.raw("getRenderBlockPass", call(b, "getRenderBlockPass"));
        l.raw("isCollidable", call(b, "isCollidable"));
        l.raw("getMobilityFlag", call(b, "getMobilityFlag"));
        l.raw("tickRate", call(b, "tickRate"));
        l.raw("getBlockName", call(b, "getBlockName"));
        StringBuilder damage = new StringBuilder("[");
        StringBuilder idDropped = new StringBuilder("[");
        for (int m = 0; m < 16; m++) {
            if (m > 0) { damage.append(','); idDropped.append(','); }
            damage.append(call(b, "damageDropped", m));
            idDropped.append(call(b, "idDropped", m, new Random(0L)));
        }
        l.raw("damageDropped", damage.append(']').toString());
        l.raw("idDropped", idDropped.append(']').toString());
        l.raw("quantityDropped", call(b, "quantityDropped", new Random(0L)));
        StringBuilder side = new StringBuilder("[");
        for (int s = 0; s < 6; s++) {
            if (s > 0) side.append(',');
            side.append(call(b, "getBlockTextureFromSide", s));
        }
        l.raw("getBlockTextureFromSide", side.append(']').toString());
        // index side * 16 + meta
        StringBuilder tex = new StringBuilder("[");
        for (int s = 0; s < 6; s++) {
            for (int m = 0; m < 16; m++) {
                if (s + m > 0) tex.append(',');
                tex.append(call(b, "getBlockTextureFromSideAndMetadata", s, m));
            }
        }
        return l.raw("getBlockTextureFromSideAndMetadata", tex.append(']').toString());
    }

    private static Json.Line staticsLine(Object[] blocksList) throws Exception {
        Json.Line l = new Json.Line().put("t", 0).put("e", "block_statics");
        // src: Block.java:17-23
        for (String name : new String[] {"tickOnLoad", "opaqueCubeLookup", "isBlockContainer", "lightOpacity",
                "canBlockGrass", "lightValue", "setNeighborNotifyOnMetadataChange"}) {
            l.raw(name, value(blockCls.getField(name).get(null)));
        }
        StringBuilder names = new StringBuilder("[");
        for (int id = 0; id < 256; id++) {
            if (id > 0) names.append(',');
            names.append(blocksList[id] == null ? "null" : call(blocksList[id], "getBlockName"));
        }
        l.raw("blockName", names.append(']').toString());
        // src: BlockFire.java:6-7, filled by initializeBlock (BlockFire.java:14-24, Block.java:700)
        Object fire = blockCls.getField("fire").get(null);
        Class<?> fireCls = fire.getClass();
        l.raw("fire_encourage", value(declared(fireCls, "chanceToEncourageFire").get(fire)));
        l.raw("fire_catch", value(declared(fireCls, "abilityToCatchFire").get(fire)));
        return l;
    }

    private static Map<String, Object> sortedMaterials() {
        Map<String, Object> m = new TreeMap<>();
        for (Map.Entry<Object, String> e : MATERIAL_NAMES.entrySet()) m.put(e.getValue(), e.getKey());
        return m;
    }

    private static Json.Line materialLine(String name, Object mat) throws Exception {
        Json.Line l = new Json.Line().put("t", 0).put("e", "material").put("name", name)
                .put("class", mat.getClass().getSimpleName());
        l.raw("fields", fields(mat, materialCls));
        // src: Material.java:43-97 and the four subclasses
        for (String m : new String[] {"getIsLiquid", "isSolid", "getCanBlockGrass", "getIsSolid", "getBurning",
                "getIsGroundCover", "getIsTranslucent", "getIsHarvestable", "getMaterialMobility"}) {
            l.raw(m, call(mat, m));
        }
        return l;
    }

    private static Json.Line mapColorLine(int i, Object c) throws Exception {
        Json.Line l = new Json.Line().put("t", 0).put("e", "map_color").put("index", i);
        if (c == null) return l.put("present", false);
        l.put("present", true);
        return l.raw("fields", fields(c, mapColorCls));
    }

    // ---- reflection and reduction ------------------------------------------------

    /** Every non-static field declared from {@code top} down to the object's class, as a JSON object. */
    private static String fields(Object o, Class<?> top) throws Exception {
        List<Class<?>> chain = new ArrayList<>();
        for (Class<?> c = o.getClass(); c != top.getSuperclass(); c = c.getSuperclass()) chain.add(0, c);
        Map<String, String> out = new TreeMap<>();
        for (Class<?> c : chain) {
            for (Field f : c.getDeclaredFields()) {
                if (Modifier.isStatic(f.getModifiers())) continue;
                f.setAccessible(true);
                if (out.put(f.getName(), value(f.get(o))) != null) {
                    throw new IllegalStateException("field name " + f.getName() + " repeats in the chain of " + o.getClass());
                }
            }
        }
        StringBuilder b = new StringBuilder("{");
        for (Map.Entry<String, String> e : out.entrySet()) {
            if (b.length() > 1) b.append(',');
            Json.quote(b, e.getKey());
            b.append(':').append(e.getValue());
        }
        return b.append('}').toString();
    }

    private static Field declared(Class<?> c, String name) throws NoSuchFieldException {
        for (Class<?> k = c; k != null; k = k.getSuperclass()) {
            try {
                Field f = k.getDeclaredField(name);
                f.setAccessible(true);
                return f;
            } catch (NoSuchFieldException e) {
                // try the superclass
            }
        }
        throw new NoSuchFieldException(c.getName() + "." + name);
    }

    /**
     * The JSON of {@code o.name(args)} (the virtual override, protected methods included), or
     * {@code {"throws":"<exception simple name>"}} when vanilla throws.
     */
    private static String call(Object o, String name, Object... args) throws Exception {
        Class<?>[] types = new Class<?>[args.length];
        for (int i = 0; i < args.length; i++) types[i] = args[i] instanceof Integer ? int.class : args[i].getClass();
        Method m = null;
        for (Class<?> k = o.getClass(); k != null && m == null; k = k.getSuperclass()) {
            try {
                m = k.getDeclaredMethod(name, types);
            } catch (NoSuchMethodException e) {
                // try the superclass
            }
        }
        if (m == null) throw new NoSuchMethodException(o.getClass().getName() + "." + name);
        m.setAccessible(true);
        try {
            return value(m.invoke(o, args));
        } catch (InvocationTargetException e) {
            StringBuilder b = new StringBuilder("{\"throws\":");
            Json.quote(b, e.getCause().getClass().getSimpleName());
            return b.append('}').toString();
        }
    }

    /** Reduction rules: tests/golden/README.md, {@code kind: "block_table"}. */
    private static String value(Object v) throws Exception {
        StringBuilder b = new StringBuilder();
        if (v == null) return "null";
        if (v instanceof Boolean || v instanceof Integer || v instanceof Long || v instanceof Short || v instanceof Byte) {
            return v.toString();
        }
        if (v instanceof Float) return quoted(Hex.f32((Float) v));
        if (v instanceof Double) return quoted(Hex.f64((Double) v));
        if (v instanceof String) return quoted((String) v);
        if (v instanceof Enum) return quoted(((Enum<?>) v).name());
        if (v instanceof Class) return quoted(((Class<?>) v).getSimpleName());
        // Unseeded `new Random()` field initialisers (census §2): only the type is dumped.
        if (v instanceof Random) return quoted("java.util.Random");
        if (blockCls.isInstance(v)) return Integer.toString(blockCls.getField("blockID").getInt(v));
        if (materialCls.isInstance(v)) {
            String n = MATERIAL_NAMES.get(v);
            if (n == null) throw new IllegalStateException("material not a Material static: " + v);
            return quoted(n);
        }
        if (mapColorCls.isInstance(v)) return Integer.toString(mapColorCls.getField("colorIndex").getInt(v));
        if (stepSoundCls.isInstance(v)) {
            b.append("{\"class\":").append(quoted(v.getClass().getSimpleName()));
            b.append(",\"name\":").append(value(stepSoundCls.getField("field_1678_a").get(v)));
            b.append(",\"volume\":").append(call(v, "getVolume"));
            b.append(",\"pitch\":").append(call(v, "getPitch"));
            b.append(",\"stepSoundDir\":").append(call(v, "stepSoundDir"));
            b.append(",\"stepSoundDir2\":").append(call(v, "stepSoundDir2"));
            return b.append('}').toString();
        }
        if (v.getClass().isArray()) {
            b.append('[');
            for (int i = 0; i < Array.getLength(v); i++) {
                if (i > 0) b.append(',');
                b.append(value(Array.get(v, i)));
            }
            return b.append(']').toString();
        }
        if (v instanceof Collection) {
            b.append('[');
            for (Object e : (Collection<?>) v) {
                if (b.length() > 1) b.append(',');
                b.append(value(e));
            }
            return b.append(']').toString();
        }
        throw new IllegalStateException("block_table: no reduction rule for " + v.getClass().getName());
    }

    private static String quoted(String s) {
        StringBuilder b = new StringBuilder();
        Json.quote(b, s);
        return b.toString();
    }
}
