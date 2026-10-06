package harness;

import static harness.Main.Scenario.list;
import static harness.Main.Scenario.str;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import net.minecraft.src.ChunkCoordIntPair;
import net.minecraft.src.ChunkCoordinates;
import net.minecraft.src.ChunkPosition;

/**
 * Tier A kind {@code java_hash_order}: {@code java.util.HashSet} iteration order on the
 * oracle JDK (tests/golden/README.md).
 *
 * <p>Keys are the vanilla classes themselves, so their {@code hashCode}/{@code equals} are
 * vanilla's: ChunkCoordIntPair.java:16-23, ChunkPosition.java:14-25,
 * ChunkCoordinates.java:23-34.
 *
 * <p>Next to the real {@code HashSet} runs a model of the JDK 8+ {@code HashMap} table
 * (capacity, bucket index, list-bin order; tests/golden/README.md documents it for the
 * Rust side). At every dump the oracle checks the real order against it and fails on a
 * mismatch. Buckets that have held a tree bin (TreeNode) since the last {@code clear} are
 * checked for bucket placement and membership only; their internal order is the JDK's
 * red-black-tree linkage, which this model does not reproduce. A tree bin orders keys by
 * hash, then {@code compareTo} for {@code Comparable<Self>} keys, then
 * {@code System.identityHashCode}, which differs between runs; the oracle therefore fails
 * if a tree bin ever holds two distinct non-{@code Comparable<Self>} keys with the same
 * hash code (none of the vanilla key classes is {@code Comparable<Self>}).
 */
final class HashOrder {
    private HashOrder() {}

    private static final int SAMPLES = 32;

    static Trace run(Map<String, Object> sc) {
        String keyType = str(sc, "key_type");
        List<Object> ops = list(sc, "ops");
        List<Object> dumpAfter = list(sc, "dump_after");
        Set<Long> dumps = new HashSet<>();
        long prev = 0;
        for (Object o : dumpAfter) {
            long n = (Long) o;
            if (n <= prev || n > ops.size()) {
                throw new IllegalArgumentException("dump_after must be strictly increasing within 1.." + ops.size());
            }
            dumps.add(n);
            prev = n;
        }

        Trace t = new Trace();
        // hash_codes: the first SAMPLES distinct keys (by equals) in op order, adds and removes alike.
        Map<Object, String> samples = new LinkedHashMap<>();
        for (Object o : ops) {
            Map<String, Object> op = op(o);
            if (!op.containsKey("key")) continue;
            Object k = key(keyType, op.get("key"));
            if (samples.size() < SAMPLES && !samples.containsKey(k)) samples.put(k, json(k));
        }
        StringBuilder s = new StringBuilder("[");
        for (Map.Entry<Object, String> e : samples.entrySet()) {
            if (s.length() > 1) s.append(',');
            s.append("{\"key\":").append(e.getValue()).append(",\"hash\":").append(e.getKey().hashCode()).append('}');
        }
        t.add(new Json.Line().put("t", 0).put("e", "hash_codes").put("key_type", keyType).raw("samples", s + "]"));

        HashSet<Object> set = new HashSet<>();   // default capacity, as every vanilla `new HashSet()`
        Model model = new Model(keyType.equals("String") || keyType.equals("Integer"));
        for (int i = 0; i < ops.size(); i++) {
            Map<String, Object> op = op(ops.get(i));
            String what = str(op, "op");
            switch (what) {
                case "add": {
                    Object k = key(keyType, op.get("key"));
                    if (set.add(k)) model.add(k);
                    break;
                }
                case "remove": {
                    Object k = key(keyType, op.get("key"));
                    if (set.remove(k)) model.remove(k);
                    break;
                }
                case "clear":
                    if (op.containsKey("key")) throw new IllegalArgumentException("clear takes no key");
                    set.clear();
                    model.clear();
                    break;
                default: throw new IllegalArgumentException("unknown op " + what);
            }
            long n = i + 1;
            if (dumps.contains(n) || n == ops.size()) t.add(dump(set, model, n));
        }
        System.out.println("harness:   " + keyType + ": final capacity " + model.capacity
                + ", tree bins created " + model.treeifications);
        return t;
    }

    private static Json.Line dump(HashSet<Object> set, Model model, long after) {
        List<Object> actual = new ArrayList<>(set);
        String problem = model.check(actual);
        if (problem != null) {
            throw new IllegalStateException("HashSet order after op " + after + " disagrees with the table model: "
                    + problem + "\n  actual " + render(actual));
        }
        StringBuilder hashes = new StringBuilder("[");
        StringBuilder keys = new StringBuilder("[");
        for (Object k : actual) {
            if (hashes.length() > 1) { hashes.append(','); keys.append(','); }
            hashes.append(k.hashCode());
            keys.append(json(k));
        }
        return new Json.Line().put("t", 0).put("e", "hash_order").put("after", after).put("size", set.size())
                .raw("hashes", hashes + "]").raw("keys", keys + "]");
    }

    private static String render(List<Object> keys) {
        StringBuilder b = new StringBuilder();
        for (Object k : keys) b.append(json(k)).append(' ');
        return b.toString();
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> op(Object o) {
        if (!(o instanceof Map)) throw new IllegalArgumentException("op must be an object");
        return (Map<String, Object>) o;
    }

    @SuppressWarnings("unchecked")
    private static Object key(String keyType, Object v) {
        switch (keyType) {
            case "ChunkCoordIntPair": {
                int[] a = ints(v, 2);
                return new ChunkCoordIntPair(a[0], a[1]);
            }
            case "ChunkPosition": {
                int[] a = ints(v, 3);
                return new ChunkPosition(a[0], a[1], a[2]);
            }
            case "ChunkCoordinates": {
                int[] a = ints(v, 3);
                return new ChunkCoordinates(a[0], a[1], a[2]);
            }
            case "Integer":
                if (!(v instanceof Long)) throw new IllegalArgumentException("Integer key must be an integer");
                return Integer.valueOf(Math.toIntExact((Long) v));
            case "String":
                if (!(v instanceof String)) throw new IllegalArgumentException("String key must be a string");
                return v;
            default: throw new IllegalArgumentException("unknown key_type " + keyType);
        }
    }

    @SuppressWarnings("unchecked")
    private static int[] ints(Object v, int n) {
        if (!(v instanceof List) || ((List<Object>) v).size() != n) {
            throw new IllegalArgumentException("key must be an array of " + n + " integers");
        }
        int[] a = new int[n];
        for (int i = 0; i < n; i++) a[i] = Math.toIntExact((Long) ((List<Object>) v).get(i));
        return a;
    }

    private static String json(Object k) {
        if (k instanceof ChunkCoordIntPair) {
            ChunkCoordIntPair p = (ChunkCoordIntPair) k;
            return "[" + p.chunkXPos + "," + p.chunkZPos + "]";
        }
        if (k instanceof ChunkPosition) {
            ChunkPosition p = (ChunkPosition) k;
            return "[" + p.x + "," + p.y + "," + p.z + "]";
        }
        if (k instanceof ChunkCoordinates) {
            ChunkCoordinates p = (ChunkCoordinates) k;
            return "[" + p.x + "," + p.y + "," + p.z + "]";
        }
        if (k instanceof Integer) return k.toString();
        StringBuilder b = new StringBuilder();
        Json.quote(b, (String) k);
        return b.toString();
    }

    /**
     * JDK 8+ {@code HashMap} table model (java.util.HashMap {@code hash}, {@code putVal},
     * {@code resize}, {@code treeifyBin}, {@code removeNode}, {@code clear}). Iteration is by
     * bucket index {@code spread(h) & (capacity - 1)} ascending; in a list bin, by insertion
     * time (appends go to the tail and a resize split keeps relative order).
     */
    private static final class Model {
        private static final class Entry {
            final Object key;
            final int spread;
            final long seq;

            Entry(Object key, long seq) {
                int h = key.hashCode();
                this.key = key;
                this.spread = h ^ (h >>> 16);
                this.seq = seq;
            }
        }

        private final boolean comparableKeys;
        private final List<Entry> entries = new ArrayList<>();
        /** Buckets that have held a tree bin since the last clear (order not modelled). */
        private final Set<Integer> treeBuckets = new HashSet<>();
        int capacity;   // 0 until the first add (the table is allocated lazily)
        int treeifications;
        private long seq;

        Model(boolean comparableKeys) {
            this.comparableKeys = comparableKeys;
        }

        void add(Object k) {
            if (capacity == 0) capacity = 16;
            Entry e = new Entry(k, seq++);
            int b = bucket(e);
            int before = 0;
            for (Entry o : entries) if (bucket(o) == b) before++;
            entries.add(e);
            if (treeBuckets.contains(b)) {
                checkTies(b);                      // putTreeVal
            } else if (before >= 8) {
                // putVal: appending to a list bin that already held TREEIFY_THRESHOLD (8) nodes
                // calls treeifyBin, which resizes instead while capacity < MIN_TREEIFY_CAPACITY (64).
                if (capacity < 64) {
                    resize();
                } else {
                    treeBuckets.add(b);
                    treeifications++;
                    checkTies(b);
                }
            }
            if (entries.size() > capacity / 4 * 3) resize();   // threshold = 0.75 * capacity
        }

        private void resize() {
            Set<Integer> split = new HashSet<>();
            for (int b : treeBuckets) { split.add(b); split.add(b + capacity); }
            treeBuckets.clear();
            treeBuckets.addAll(split);
            capacity *= 2;
        }

        private void checkTies(int b) {
            if (comparableKeys) return;
            Set<Integer> seen = new HashSet<>();
            for (Entry o : entries) {
                if (bucket(o) == b && !seen.add(o.spread)) {
                    throw new IllegalStateException("tree bin " + b + " holds distinct keys with hash code "
                            + o.key.hashCode() + "; their order would depend on identityHashCode");
                }
            }
        }

        void remove(Object k) {
            entries.removeIf(e -> e.key.equals(k));
        }

        void clear() {
            entries.clear();   // HashMap.clear keeps the table, so the capacity stays
            treeBuckets.clear();
        }

        private int bucket(Entry e) {
            return e.spread & (capacity - 1);
        }

        /** Null if {@code actual} (the real iteration order) agrees with the model. */
        String check(List<Object> actual) {
            if (actual.size() != entries.size()) return "size " + actual.size() + " vs model " + entries.size();
            Map<Object, Entry> byKey = new java.util.IdentityHashMap<>();
            for (Entry e : entries) byKey.put(e.key, e);
            int lastBucket = -1;
            Entry lastEntry = null;
            for (Object k : actual) {
                Entry e = byKey.get(k);
                if (e == null) return "key " + json(k) + " not in the model";
                int b = bucket(e);
                if (b < lastBucket) return "bucket " + b + " after bucket " + lastBucket;
                if (b == lastBucket && !treeBuckets.contains(b) && e.seq < lastEntry.seq) {
                    return "list bin " + b + " not in insertion order at " + json(k);
                }
                lastBucket = b;
                lastEntry = e;
            }
            return null;
        }
    }
}
