package harness;

import static harness.Main.Scenario.str;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.zip.GZIPInputStream;
import net.minecraft.src.CompressedStreamTools;
import net.minecraft.src.NBTBase;
import net.minecraft.src.NBTTagByte;
import net.minecraft.src.NBTTagByteArray;
import net.minecraft.src.NBTTagCompound;
import net.minecraft.src.NBTTagDouble;
import net.minecraft.src.NBTTagFloat;
import net.minecraft.src.NBTTagInt;
import net.minecraft.src.NBTTagList;
import net.minecraft.src.NBTTagLong;
import net.minecraft.src.NBTTagShort;
import net.minecraft.src.NBTTagString;

/**
 * Tier A kinds {@code nbt_write} and {@code nbt_read} (tests/golden/README.md).
 *
 * <p>Everything goes through the vanilla classes: trees are built with
 * {@code NBTTagCompound.setTag} (NBTTagCompound.java:43) and {@code NBTTagList.setTag}
 * (NBTTagList.java:50), written with {@code CompressedStreamTools.write}
 * (CompressedStreamTools.java:47, i.e. {@code NBTBase.writeTag}, NBTBase.java:37) and read
 * with {@code CompressedStreamTools.read} (CompressedStreamTools.java:38). The gzip layer is
 * {@code writeGzippedCompoundToOutputStream} / {@code loadGzippedCompoundFromOutputStream}
 * (CompressedStreamTools.java:27 / :14).
 */
final class Nbt {
    private Nbt() {}

    /** Type names in tag-id order (NBTBase.java:45-71). */
    private static final String[] TYPES = {
        "end", "byte", "short", "int", "long", "float", "double", "byte_array", "string", "list", "compound"
    };

    // ---- nbt_write -----------------------------------------------------------

    @SuppressWarnings("unchecked")
    static Trace write(Map<String, Object> sc) throws IOException {
        Map<String, Object> rootSpec = (Map<String, Object>) sc.get("root");
        if (rootSpec == null) throw new IllegalArgumentException("missing \"root\"");
        if (!"compound".equals(str(rootSpec, "type"))) {
            throw new IllegalArgumentException("root must be a compound (CompressedStreamTools.java:38-45)");
        }
        NBTTagCompound root = (NBTTagCompound) build("compound", rootSpec.get("value"));
        root.setKey(str(rootSpec, "name"));

        Trace t = new Trace();
        byte[] bytes = serialise(root);
        t.add(bytesLine(bytes));
        // Canonical form after the write: an empty list's element type is what the write chose.
        String tree = canonicalRoot(root);
        t.add(new Json.Line().put("t", 0).put("e", "nbt_tree").raw("root", tree));
        t.add(roundtripLine(tree, bytes));

        ByteArrayOutputStream gz = new ByteArrayOutputStream();
        CompressedStreamTools.writeGzippedCompoundToOutputStream(root, gz);
        NBTTagCompound back = CompressedStreamTools.loadGzippedCompoundFromOutputStream(new ByteArrayInputStream(gz.toByteArray()));
        byte[] again = serialise(back);
        t.add(new Json.Line().put("t", 0).put("e", "nbt_gzip")
                .put("blake3_of_uncompressed_after_gunzip", Blake3.hex(again)).put("len", again.length));
        return t;
    }

    // ---- nbt_read ------------------------------------------------------------

    static Trace read(Map<String, Object> sc, Path scenarioDir) throws IOException {
        String file = str(sc, "file");
        if (file.contains("/") || file.contains("\\") || file.equals("expected.jsonl") || file.equals("scenario.json")) {
            throw new IllegalArgumentException("\"file\" must be a plain sidecar name, got " + file);
        }
        String compressed = str(sc, "compressed");
        byte[] input = Files.readAllBytes(scenarioDir.resolve(file));
        byte[] uncompressed;
        NBTTagCompound root;
        switch (compressed) {
            case "gzip":
                try (InputStream in = new GZIPInputStream(new ByteArrayInputStream(input))) {
                    uncompressed = in.readAllBytes();
                }
                root = CompressedStreamTools.loadGzippedCompoundFromOutputStream(new ByteArrayInputStream(input));
                break;
            case "none":
                uncompressed = input;
                root = CompressedStreamTools.read(new DataInputStream(new ByteArrayInputStream(input)));
                break;
            default: throw new IllegalArgumentException("unknown \"compressed\" " + compressed);
        }

        Trace t = new Trace();
        t.add(new Json.Line().put("t", 0).put("e", "nbt_file").put("file", file).put("compressed", compressed)
                .put("len", input.length).put("blake3", Blake3.hex(input))
                .put("uncompressed_len", uncompressed.length).put("uncompressed_blake3", Blake3.hex(uncompressed)));
        byte[] bytes = serialise(root);
        t.add(bytesLine(bytes));
        String tree = canonicalRoot(root);
        t.add(new Json.Line().put("t", 0).put("e", "nbt_tree").raw("root", tree));
        t.add(roundtripLine(tree, bytes));
        // The input is an input, but regen.sh replaces every generated file in the scenario
        // directory; re-emitting it unchanged keeps it there.
        t.sidecars.put(file, input);
        return t;
    }

    // ---- shared --------------------------------------------------------------

    private static byte[] serialise(NBTTagCompound root) throws IOException {
        ByteArrayOutputStream buf = new ByteArrayOutputStream();
        DataOutputStream out = new DataOutputStream(buf);
        CompressedStreamTools.write(root, out);
        out.flush();
        return buf.toByteArray();
    }

    private static Json.Line bytesLine(byte[] bytes) {
        return new Json.Line().put("t", 0).put("e", "nbt_bytes").put("len", bytes.length)
                .put("blake3", Blake3.hex(bytes)).put("bytes", Hex.of(bytes));
    }

    /** Re-reads {@code bytes} with the vanilla reader and compares canonical trees. */
    private static Json.Line roundtripLine(String tree, byte[] bytes) throws IOException {
        NBTTagCompound reread = CompressedStreamTools.read(new DataInputStream(new ByteArrayInputStream(bytes)));
        String again = canonicalRoot(reread);
        Json.Line l = new Json.Line().put("t", 0).put("e", "nbt_roundtrip").put("equal", tree.equals(again));
        if (!tree.equals(again)) l.raw("reread", again);
        return l;
    }

    // ---- scenario tree -> vanilla tags ---------------------------------------

    @SuppressWarnings("unchecked")
    private static NBTBase build(String type, Object v) {
        switch (type) {
            case "byte": return new NBTTagByte((byte) intIn(v, Byte.MIN_VALUE, Byte.MAX_VALUE));
            case "short": return new NBTTagShort((short) intIn(v, Short.MIN_VALUE, Short.MAX_VALUE));
            case "int": return new NBTTagInt((int) intIn(v, Integer.MIN_VALUE, Integer.MAX_VALUE));
            case "long": return new NBTTagLong(intIn(v, Long.MIN_VALUE, Long.MAX_VALUE));
            case "float": return new NBTTagFloat(Hex.parseF32(string(v)));
            case "double": return new NBTTagDouble(Hex.parseF64(string(v)));
            case "byte_array": return new NBTTagByteArray(Hex.parse(string(v)));
            case "string": return new NBTTagString(checkString(string(v)));
            case "list": {
                Map<String, Object> m = (Map<String, Object>) v;
                String elem = str(m, "elem");
                typeId(elem);
                if (elem.equals("end")) throw new IllegalArgumentException("list elem \"end\" cannot be built");
                NBTTagList list = new NBTTagList();
                Object items = m.get("items");
                if (!(items instanceof List)) throw new IllegalArgumentException("list needs \"items\"");
                for (Object item : (List<Object>) items) list.setTag(build(elem, item));
                return list;
            }
            case "compound": {
                if (!(v instanceof Map)) throw new IllegalArgumentException("compound value must be an object");
                NBTTagCompound c = new NBTTagCompound();
                // Scenario key order (the JSON reader keeps it) is the insertion order.
                for (Map.Entry<String, Object> e : ((Map<String, Object>) v).entrySet()) {
                    Map<String, Object> node = (Map<String, Object>) e.getValue();
                    if (node.containsKey("name")) throw new IllegalArgumentException("\"name\" only on the root");
                    c.setTag(checkString(e.getKey()), build(str(node, "type"), node.get("value")));
                }
                return c;
            }
            default: throw new IllegalArgumentException("unknown or unbuildable NBT type " + type);
        }
    }

    private static long intIn(Object v, long min, long max) {
        if (!(v instanceof Long)) throw new IllegalArgumentException("expected an integer, got " + v);
        long x = (Long) v;
        if (x < min || x > max) throw new IllegalArgumentException(x + " out of range");
        return x;
    }

    private static String string(Object v) {
        if (!(v instanceof String)) throw new IllegalArgumentException("expected a string, got " + v);
        return (String) v;
    }

    /** UTF-8 trace output cannot carry an unpaired surrogate; refuse one instead of mangling it. */
    private static String checkString(String s) {
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (Character.isHighSurrogate(c) && i + 1 < s.length() && Character.isLowSurrogate(s.charAt(i + 1))) {
                i++;
            } else if (Character.isSurrogate(c)) {
                throw new IllegalArgumentException("unpaired surrogate in NBT string");
            }
        }
        return s;
    }

    private static int typeId(String type) {
        for (int i = 0; i < TYPES.length; i++) if (TYPES[i].equals(type)) return i;
        throw new IllegalArgumentException("unknown NBT type " + type);
    }

    // ---- vanilla tags -> canonical JSON --------------------------------------

    /** {@code {"type":"compound","name":..,"value":{..}}}. */
    static String canonicalRoot(NBTTagCompound root) {
        StringBuilder b = new StringBuilder("{\"type\":\"compound\",\"name\":");
        Json.quote(b, checkString(root.getKey()));
        b.append(",\"value\":");
        value(b, root);
        return b.append('}').toString();
    }

    private static void value(StringBuilder b, NBTBase tag) {
        switch (tag.getType()) {
            case 1: b.append(((NBTTagByte) tag).byteValue); break;
            case 2: b.append(((NBTTagShort) tag).shortValue); break;
            case 3: b.append(((NBTTagInt) tag).intValue); break;
            case 4: b.append(((NBTTagLong) tag).longValue); break;
            case 5: Json.quote(b, Hex.f32(((NBTTagFloat) tag).floatValue)); break;
            case 6: Json.quote(b, Hex.f64(((NBTTagDouble) tag).doubleValue)); break;
            case 7: Json.quote(b, Hex.of(((NBTTagByteArray) tag).byteArray)); break;
            case 8: Json.quote(b, checkString(((NBTTagString) tag).stringValue)); break;
            case 9: {
                NBTTagList l = (NBTTagList) tag;
                b.append("{\"elem\":\"").append(TYPES[listElemType(l)]).append("\",\"items\":[");
                for (int i = 0; i < l.tagCount(); i++) {
                    if (i > 0) b.append(',');
                    value(b, l.tagAt(i));
                }
                b.append("]}");
                break;
            }
            case 10: {
                // Children sorted by Java String natural order (UTF-16 code units).
                TreeMap<String, NBTBase> sorted = new TreeMap<>();
                for (Object o : ((NBTTagCompound) tag).getTags()) {
                    NBTBase child = (NBTBase) o;
                    if (sorted.put(child.getKey(), child) != null) throw new IllegalStateException("duplicate key");
                }
                b.append('{');
                boolean first = true;
                for (Map.Entry<String, NBTBase> e : sorted.entrySet()) {
                    if (!first) b.append(',');
                    first = false;
                    Json.quote(b, checkString(e.getKey()));
                    b.append(":{\"type\":\"").append(TYPES[e.getValue().getType()]).append("\",\"value\":");
                    value(b, e.getValue());
                    b.append('}');
                }
                b.append('}');
                break;
            }
            default: throw new IllegalStateException("tag type " + tag.getType() + " has no value");
        }
    }

    /**
     * The element type the list is written with: the first item's type when non-empty
     * (NBTTagList.java:14-15), else the stored {@code tagType} (NBTTagList.java:11), which a
     * write sets to 1 (NBTTagList.java:17) and a read sets from the stream (NBTTagList.java:30).
     */
    private static int listElemType(NBTTagList l) {
        if (l.tagCount() > 0) {
            int first = l.tagAt(0).getType();
            for (int i = 1; i < l.tagCount(); i++) {
                if (l.tagAt(i).getType() != first) throw new IllegalStateException("mixed list element types");
            }
            return first;
        }
        try {
            Field f = NBTTagList.class.getDeclaredField("tagType");
            f.setAccessible(true);
            int type = f.getByte(l);
            if (type < 0 || type >= TYPES.length) throw new IllegalStateException("list element type " + type);
            return type;
        } catch (ReflectiveOperationException e) {
            throw new IllegalStateException(e);
        }
    }
}
