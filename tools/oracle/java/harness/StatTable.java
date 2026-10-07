package harness;

import static harness.Main.Scenario.list;

import java.io.File;
import java.lang.reflect.Field;
import java.net.URL;
import java.net.URLClassLoader;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import java.util.TreeMap;

/**
 * Tier A kind {@code stat_table} (WP-2.7; tests/golden/README.md "kind: stat_table"): the
 * {@code StatList} and {@code AchievementList} registries after vanilla's static initialisers,
 * read by reflection, plus the three {@code IStatType} formats for the scenario's sample values.
 *
 * <p>Stat names are translated strings ({@code StatCollector}, from the jar's {@code lang/}), which
 * neither the port nor the fixtures may hold (PLAN_v4 D8). So the dump runs the static
 * initialisers again in a fresh class loader whose {@code StringTranslate} table has been
 * replaced, before {@code Block} initialises, by {@link KeyTable}: every key translates to itself,
 * and a key whose en_US text is a format ({@code "stat.mineBlock=%1$s Mined"}) to
 * {@code key(%1$s)}, so a formatted name shows its argument's key:
 * {@code stat.mineBlock(tile.stone.name)}. The fresh loader also makes the dump independent of
 * whichever scenario initialised the vanilla classes first in this JVM; its first class is
 * {@code Block}, which is vanilla's order (StatList's initialiser runs from Item's, from Block's;
 * Block.java:689, Item.java:380, Block.java:705).
 */
final class StatTable {
    private StatTable() {}

    private static final String P = "net.minecraft.src.";

    static Trace run(Map<String, Object> sc) throws Exception {
        ClassLoader loader = freshLoader();
        Class<?> st = Class.forName(P + "StringTranslate", true, loader);
        Object instance = field(st, "instance").get(null);
        Field table = field(st, "translateTable");
        table.set(instance, new KeyTable((Properties) table.get(instance)));
        // StatBase's DecimalFormat("########0.00") takes the default locale's symbols at class
        // initialisation (StatBase.java:71); pin them to en_US for the dump, then restore.
        Locale saved = Locale.getDefault();
        Locale.setDefault(Locale.US);
        try {
            Class.forName(P + "Block", true, loader);
        } finally {
            Locale.setDefault(saved);
        }
        Class<?> statList = Class.forName(P + "StatList", true, loader);
        Class<?> achievementList = Class.forName(P + "AchievementList", true, loader);
        Class<?> statBase = Class.forName(P + "StatBase", true, loader);

        List<?> listed = (List<?>) field(statList, "field_25188_a").get(null);
        Map<?, ?> oneShot = (Map<?, ?>) field(statList, "oneShotStats").get(null);
        List<?> general = (List<?>) field(statList, "generalStats").get(null);
        List<?> itemStats = (List<?>) field(statList, "itemStats").get(null);
        List<?> mineStats = (List<?>) field(statList, "objectMineStats").get(null);
        List<?> achievements = (List<?>) field(achievementList, "achievementList").get(null);

        Trace t = new Trace();
        t.add(new Json.Line().put("t", 0).put("e", "stat_count").put("listed", listed.size())
                .put("oneShot", oneShot.size()).put("generalStats", general.size())
                .put("itemStats", itemStats.size()).put("objectMineStats", mineStats.size())
                .put("achievements", achievements.size()));
        IdentityHashMap<Object, Boolean> inList = new IdentityHashMap<>();
        for (int i = 0; i < listed.size(); i++) {
            Object s = listed.get(i);
            inList.put(s, Boolean.TRUE);
            t.add(statLine(new Json.Line().put("t", 0).put("e", "stat").put("i", i), s));
        }
        TreeMap<Integer, Object> unlisted = new TreeMap<>();
        for (Object s : oneShot.values()) {
            if (!inList.containsKey(s)) unlisted.put(getInt(s, "statId"), s);
        }
        for (Object s : unlisted.values()) {
            t.add(statLine(new Json.Line().put("t", 0).put("e", "unlisted_stat"), s));
        }
        t.add(listLine("generalStats", general));
        t.add(listLine("itemStats", itemStats));
        t.add(listLine("objectMineStats", mineStats));
        for (String a : new String[] {"mineBlockStatArray", "objectUseStats", "objectBreakStats", "objectCraftStats"}) {
            t.add(arrayLine(a, (Object[]) field(statList, a).get(null)));
        }
        for (int i = 0; i < achievements.size(); i++) {
            Object a = achievements.get(i);
            Object parent = get(a, "parentAchievement");
            Object icon = get(a, "theItemStack");
            Json.Line l = new Json.Line().put("t", 0).put("e", "achievement").put("i", i)
                    .put("statId", getInt(a, "statId")).put("name", (String) get(a, "statName"))
                    .put("column", getInt(a, "displayColumn")).put("row", getInt(a, "displayRow"));
            if (parent == null) l.raw("parent", "null");
            else l.put("parent", getInt(parent, "statId"));
            l.raw("icon", new Json.Line().put("id", getInt(icon, "itemID"))
                            .put("damage", getInt(icon, "itemDamage")).toString())
                    .put("isSpecial", (Boolean) get(a, "isSpecial"))
                    .put("isIndependent", (Boolean) get(a, "isIndependent"));
            t.add(l);
        }
        t.add(new Json.Line().put("t", 0).put("e", "achievement_bounds")
                .put("minDisplayColumn", field(achievementList, "minDisplayColumn").getInt(null))
                .put("minDisplayRow", field(achievementList, "minDisplayRow").getInt(null))
                .put("maxDisplayColumn", field(achievementList, "maxDisplayColumn").getInt(null))
                .put("maxDisplayRow", field(achievementList, "maxDisplayRow").getInt(null)));
        Object[][] types = {
            {"StatTypeSimple", field(statBase, "field_27087_i").get(null)},
            {"StatTypeTime", field(statBase, "timeStatType").get(null)},
            {"StatTypeDistance", field(statBase, "distanceStatType").get(null)},
        };
        if (sc.containsKey("format_values")) {
            for (Object[] ty : types) {
                java.lang.reflect.Method format = ty[1].getClass().getMethod("format", int.class);
                format.setAccessible(true);
                for (Object o : list(sc, "format_values")) {
                    int v = Math.toIntExact(((Number) o).longValue());
                    t.add(new Json.Line().put("t", 0).put("e", "stat_format").put("type", (String) ty[0])
                            .put("v", v).put("s", (String) format.invoke(ty[1], v)));
                }
            }
        }
        return t;
    }

    /** The identity translation table described in the class comment. */
    static final class KeyTable extends Properties {
        private final Properties real;

        KeyTable(Properties real) {
            this.real = real;
        }

        @Override
        public String getProperty(String key) {
            return getProperty(key, null);
        }

        @Override
        public String getProperty(String key, String defaultValue) {
            String text = real.getProperty(key);
            return text != null && text.indexOf('%') >= 0 ? key + "(%1$s)" : key;
        }
    }

    /** A class loader over the oracle's class path that shares nothing with this one. */
    private static ClassLoader freshLoader() throws Exception {
        List<URL> urls = new ArrayList<>();
        for (String p : System.getProperty("java.class.path").split(File.pathSeparator)) {
            urls.add(new File(p).toURI().toURL());
        }
        return new URLClassLoader(urls.toArray(new URL[0]), ClassLoader.getPlatformClassLoader());
    }

    private static Json.Line statLine(Json.Line l, Object s) throws Exception {
        Object type = get(s, "field_26902_a");
        l.put("statId", getInt(s, "statId")).put("name", (String) get(s, "statName"))
                .put("class", s.getClass().getSimpleName())
                .put("type", type.getClass().getSimpleName())
                .put("isIndependent", (Boolean) get(s, "isIndependent"));
        if (s.getClass().getSimpleName().equals("StatCrafting")) l.put("itemId", getInt(s, "field_25073_a"));
        return l;
    }

    private static Json.Line listLine(String name, List<?> stats) throws Exception {
        StringBuilder b = new StringBuilder("[");
        for (int i = 0; i < stats.size(); i++) {
            if (i > 0) b.append(',');
            b.append(getInt(stats.get(i), "statId"));
        }
        return new Json.Line().put("t", 0).put("e", "stat_list").put("list", name).raw("ids", b.append(']').toString());
    }

    /** The non-null entries as {@code [index, statId]} pairs, in index order. */
    private static Json.Line arrayLine(String name, Object[] a) throws Exception {
        StringBuilder b = new StringBuilder("[");
        boolean first = true;
        for (int i = 0; i < a.length; i++) {
            if (a[i] == null) continue;
            if (!first) b.append(',');
            first = false;
            b.append('[').append(i).append(',').append(getInt(a[i], "statId")).append(']');
        }
        return new Json.Line().put("t", 0).put("e", "stat_array").put("array", name).put("length", a.length)
                .raw("entries", b.append(']').toString());
    }

    /** The field {@code name} declared by the object's class or a superclass. */
    private static Field field(Class<?> c, String name) throws NoSuchFieldException {
        for (Class<?> k = c; k != null; k = k.getSuperclass()) {
            try {
                Field f = k.getDeclaredField(name);
                f.setAccessible(true);
                return f;
            } catch (NoSuchFieldException e) {
                // try the superclass
            }
        }
        throw new NoSuchFieldException(c.getName() + "#" + name);
    }

    private static Object get(Object o, String name) throws Exception {
        return field(o.getClass(), name).get(o);
    }

    private static int getInt(Object o, String name) throws Exception {
        return field(o.getClass(), name).getInt(o);
    }
}
