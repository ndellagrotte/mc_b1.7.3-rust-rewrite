package harness;

import static harness.Main.Scenario.list;
import static harness.Main.Scenario.num;

import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.concurrent.Callable;
import net.minecraft.src.Block;
import net.minecraft.src.EnumToolMaterial;
import net.minecraft.src.Item;
import net.minecraft.src.ItemStack;
import net.minecraft.src.Material;
import net.minecraft.src.NBTBase;
import net.minecraft.src.NBTTagCompound;

/**
 * Tier A kind {@code item_table} (WP-1.2; tests/golden/README.md "kind: item_table"): every
 * {@code Item.itemsList} entry after the static initialisers of Block and Item, by reflection
 * plus the results of the item's data-only methods; every {@code EnumToolMaterial} constant;
 * the block texture lookups the item icons are built from; and {@code ItemStack} value
 * operations (NBT write/read, damage) on the scenario's sample stacks.
 *
 * <p>Reflection reduction rules (one JSON value per field): {@code int}/{@code short}/
 * {@code byte}/{@code long} as numbers, {@code boolean} as booleans, {@code float}/{@code double}
 * as bit-pattern strings, {@code String} as strings, {@code null} as null, {@code Block} as its
 * {@code blockID}, {@code Item} as its {@code shiftedIndex}, {@code ItemStack} as
 * {@code {"id","count","damage"}}, {@code Material} as the name of the {@code Material} static
 * field holding that instance, enums as {@code name()}, arrays as lists of reduced elements. Any
 * other type fails the scenario (add a rule, never drop a field).
 */
final class ItemTable {
    private ItemTable() {}

    static Trace run(Map<String, Object> sc) throws Exception {
        // Block's initialiser first (it creates the ItemBlocks and triggers Item's), then
        // Item.itemRand reseeded with mix(0, -1); nothing here draws from it.
        HarnessWorld.resetStatics(0L);
        Trace t = new Trace();

        Item[] items = Item.itemsList;
        int top = -1;
        for (int i = 0; i < items.length; i++) if (items[i] != null) top = i;
        for (int id = 0; id <= top; id++) t.add(itemLine(id, items[id]));

        for (EnumToolMaterial m : EnumToolMaterial.values()) {
            t.add(new Json.Line().put("t", 0).put("e", "tool_material").put("name", m.name())
                .put("ordinal", m.ordinal())
                .raw("fields", fieldsOf(EnumToolMaterial.class, m))
                .put("getMaxUses", m.getMaxUses())
                .f32("getEfficiencyOnProperMaterial", m.getEfficiencyOnProperMaterial())
                .put("getDamageVsEntity", m.getDamageVsEntity())
                .put("getHarvestLevel", m.getHarvestLevel()));
        }

        // The piston base textures read the singleton's mutable bounds
        // (BlockPistonBase.java:24-25), which a tick scenario run earlier in
        // this JVM may have left extended; resetStatics does not restore
        // them. setBlockBoundsForItemRender (BlockPistonBase.java:203-205)
        // gives the full cube, i.e. the static-init state (Block.java ctor).
        Block.pistonBase.setBlockBoundsForItemRender();
        Block.pistonStickyBase.setBlockBoundsForItemRender();
        for (int b = 0; b < 256; b++) {
            Block blk = Block.blocksList[b];
            if (blk == null) continue;
            StringBuilder side = new StringBuilder("[");
            StringBuilder sideMeta = new StringBuilder("[");
            for (int s = 0; s < 6; s++) {
                final int fs = s;
                if (s > 0) { side.append(','); sideMeta.append(','); }
                side.append(call(() -> Integer.toString(blk.getBlockTextureFromSide(fs))));
                sideMeta.append('[');
                for (int m = 0; m < 16; m++) {
                    final int fm = m;
                    if (m > 0) sideMeta.append(',');
                    sideMeta.append(call(() -> Integer.toString(blk.getBlockTextureFromSideAndMetadata(fs, fm))));
                }
                sideMeta.append(']');
            }
            t.add(new Json.Line().put("t", 0).put("e", "block_texture").put("id", b)
                .raw("getBlockTextureFromSide", side.append(']').toString())
                .raw("getBlockTextureFromSideAndMetadata", sideMeta.append(']').toString()));
        }

        int i = 0;
        for (Object o : list(sc, "stacks")) {
            @SuppressWarnings("unchecked")
            Map<String, Object> s = (Map<String, Object>) o;
            t.add(stackLine(i++, s));
        }
        return t;
    }

    // ---- items ---------------------------------------------------------------

    private static Json.Line itemLine(int id, Item it) throws Exception {
        Json.Line l = new Json.Line().put("t", 0).put("e", "item").put("id", id);
        if (it == null) return l.put("present", false);
        l.put("present", true).put("class", it.getClass().getSimpleName());

        StringBuilder f = new StringBuilder("{");
        for (Class<?> c = it.getClass(); ; c = c.getSuperclass()) {
            if (f.length() > 1) f.append(',');
            Json.quote(f, c.getSimpleName());
            f.append(':').append(fieldsOf(c, it));
            if (c == Item.class) break;
        }
        l.raw("fields", f.append('}').toString());

        StringBuilder m = new StringBuilder("{");
        method(m, "getIconFromDamage", perDamage(d -> Integer.toString(it.getIconFromDamage(d))));
        method(m, "getItemNameIS", perDamage(d -> str(it.getItemNameIS(new ItemStack(it, 1, d)))));
        method(m, "getPlacedBlockMetadata", perDamage(d -> Integer.toString(it.getPlacedBlockMetadata(d))));
        method(m, "getColorFromDamage", perDamage(d -> Integer.toString(it.getColorFromDamage(d))));
        method(m, "getItemName", call(() -> str(it.getItemName())));
        method(m, "getItemStackLimit", call(() -> Integer.toString(it.getItemStackLimit())));
        method(m, "getMaxDamage", call(() -> Integer.toString(it.getMaxDamage())));
        method(m, "isDamagable", call(() -> Boolean.toString(it.isDamagable())));
        method(m, "isFull3D", call(() -> Boolean.toString(it.isFull3D())));
        method(m, "shouldRotateAroundWhenRendering", call(() -> Boolean.toString(it.shouldRotateAroundWhenRendering())));
        method(m, "getHasSubtypes", call(() -> Boolean.toString(it.getHasSubtypes())));
        method(m, "getContainerItem", call(() -> it.getContainerItem() == null ? "null" : Integer.toString(it.getContainerItem().shiftedIndex)));
        method(m, "hasContainerItem", call(() -> Boolean.toString(it.hasContainerItem())));
        method(m, "getDamageVsEntity", call(() -> Integer.toString(it.getDamageVsEntity(null))));
        ItemStack stack = new ItemStack(it, 1, 0);
        method(m, "getStrVsBlock", perBlock(b -> f32(it.getStrVsBlock(stack, b))));
        method(m, "canHarvestBlock", perBlock(b -> Boolean.toString(it.canHarvestBlock(b))));
        return l.raw("methods", m.append('}').toString());
    }

    private interface IntFn {
        String apply(int v) throws Exception;
    }

    private interface BlockFn {
        String apply(Block b) throws Exception;
    }

    /** {@code [f(0), ..., f(15)]}, each entry exception-captured. */
    private static String perDamage(IntFn f) {
        StringBuilder b = new StringBuilder("[");
        for (int d = 0; d < 16; d++) {
            final int fd = d;
            if (d > 0) b.append(',');
            b.append(call(() -> f.apply(fd)));
        }
        return b.append(']').toString();
    }

    /** 256 entries, one per block id: {@code f(Block.blocksList[id])}, null for absent blocks. */
    private static String perBlock(BlockFn f) {
        StringBuilder b = new StringBuilder("[");
        for (int id = 0; id < 256; id++) {
            Block blk = Block.blocksList[id];
            if (id > 0) b.append(',');
            b.append(blk == null ? "null" : call(() -> f.apply(blk)));
        }
        return b.append(']').toString();
    }

    private static void method(StringBuilder b, String name, String json) {
        if (b.length() > 1) b.append(',');
        Json.quote(b, name);
        b.append(':').append(json);
    }

    /** The JSON of {@code c.call()}, or {@code {"exception":"<simple class name>"}}. */
    private static String call(Callable<String> c) {
        try {
            return c.call();
        } catch (Exception e) {
            StringBuilder b = new StringBuilder("{\"exception\":");
            Json.quote(b, e.getClass().getSimpleName());
            return b.append('}').toString();
        }
    }

    private static String str(String s) {
        if (s == null) return "null";
        StringBuilder b = new StringBuilder();
        Json.quote(b, s);
        return b.toString();
    }

    private static String f32(float v) {
        return str(Hex.f32(v));
    }

    // ---- reflection ----------------------------------------------------------

    /** {@code {name: reduced value}} over {@code c}'s own non-static, non-synthetic fields, sorted by name. */
    private static String fieldsOf(Class<?> c, Object target) throws Exception {
        Field[] fs = c.getDeclaredFields();
        Arrays.sort(fs, Comparator.comparing(Field::getName));
        StringBuilder b = new StringBuilder("{");
        for (Field f : fs) {
            if (Modifier.isStatic(f.getModifiers()) || f.isSynthetic()) continue;
            f.setAccessible(true);
            if (b.length() > 1) b.append(',');
            Json.quote(b, f.getName());
            b.append(':').append(reduce(f.get(target)));
        }
        return b.append('}').toString();
    }

    private static String reduce(Object v) throws Exception {
        if (v == null) return "null";
        if (v instanceof Integer || v instanceof Short || v instanceof Byte || v instanceof Long) return v.toString();
        if (v instanceof Boolean) return v.toString();
        if (v instanceof Float) return f32((Float) v);
        if (v instanceof Double) return str(Hex.f64((Double) v));
        if (v instanceof String) return str((String) v);
        if (v instanceof Block) return Integer.toString(((Block) v).blockID);
        if (v instanceof Item) return Integer.toString(((Item) v).shiftedIndex);
        if (v instanceof ItemStack) {
            ItemStack s = (ItemStack) v;
            return "{\"id\":" + s.itemID + ",\"count\":" + s.stackSize + ",\"damage\":" + s.getItemDamage() + "}";
        }
        if (v instanceof Enum) return str(((Enum<?>) v).name());
        if (v instanceof Material) return str(materialName((Material) v));
        if (v.getClass().isArray()) {
            StringBuilder b = new StringBuilder("[");
            int n = java.lang.reflect.Array.getLength(v);
            for (int i = 0; i < n; i++) {
                if (i > 0) b.append(',');
                b.append(reduce(java.lang.reflect.Array.get(v, i)));
            }
            return b.append(']').toString();
        }
        throw new IllegalStateException("item_table: no reduction rule for " + v.getClass().getName());
    }

    private static String materialName(Material m) throws Exception {
        for (Field f : Material.class.getDeclaredFields()) {
            if (Modifier.isStatic(f.getModifiers()) && f.getType() == Material.class && f.get(null) == m) return f.getName();
        }
        throw new IllegalStateException("item_table: Material instance not held by a Material static field");
    }

    // ---- ItemStack -----------------------------------------------------------

    private static Json.Line stackLine(int i, Map<String, Object> s) {
        int id = Math.toIntExact(num(s, "id"));
        int count = Math.toIntExact(num(s, "count"));
        int damage = Math.toIntExact(num(s, "damage"));
        ItemStack st = new ItemStack(id, count, damage);
        Json.Line l = new Json.Line().put("t", 0).put("e", "item_stack").put("i", i)
            .put("id", id).put("count", count).put("damage", damage);

        NBTTagCompound nbt = st.writeToNBT(new NBTTagCompound());
        StringBuilder types = new StringBuilder("{");
        List<NBTBase> tags = new ArrayList<>();
        for (Object o : nbt.getTags()) tags.add((NBTBase) o);
        tags.sort(Comparator.comparing(NBTBase::getKey));
        for (NBTBase tag : tags) {
            if (types.length() > 1) types.append(',');
            Json.quote(types, tag.getKey());
            types.append(':').append(tag.getType());
        }
        l.raw("nbt_types", types.append('}').toString());
        l.raw("nbt", "{\"id\":" + nbt.getShort("id") + ",\"Count\":" + nbt.getByte("Count") + ",\"Damage\":" + nbt.getShort("Damage") + "}");
        l.raw("read", stackJson(new ItemStack(nbt)));
        l.raw("copy", stackJson(st.copy()));

        l.raw("getMaxStackSize", call(() -> Integer.toString(st.getMaxStackSize())));
        l.raw("isStackable", call(() -> Boolean.toString(st.isStackable())));
        l.raw("isItemStackDamageable", call(() -> Boolean.toString(st.isItemStackDamageable())));
        l.raw("isItemDamaged", call(() -> Boolean.toString(st.isItemDamaged())));
        l.raw("getHasSubtypes", call(() -> Boolean.toString(st.getHasSubtypes())));
        l.raw("getMaxDamage", call(() -> Integer.toString(st.getMaxDamage())));
        l.raw("getItemName", call(() -> str(st.getItemName())));
        l.raw("getIconIndex", call(() -> Integer.toString(st.getIconIndex())));
        l.raw("getDamageVsEntity", call(() -> Integer.toString(st.getDamageVsEntity(null))));

        // damageItem(n, null) in list order on one copy; the data part of ItemStack.java:124-141
        // (a null entity is not an EntityPlayer, so no stat).
        ItemStack d = st.copy();
        StringBuilder after = new StringBuilder("[");
        for (Object n : list(s, "damage_by")) {
            if (after.length() > 1) after.append(',');
            int amount = Math.toIntExact((Long) n);
            after.append(call(() -> {
                d.damageItem(amount, null);
                return stackJson(d);
            }));
        }
        l.raw("damageItem", after.append(']').toString());

        ItemStack sp = st.copy();
        ItemStack part = sp.splitStack(Math.toIntExact(num(s, "split")));
        l.raw("splitStack", "[" + stackJson(sp) + "," + stackJson(part) + "]");
        return l;
    }

    private static String stackJson(ItemStack s) {
        return "{\"id\":" + s.itemID + ",\"count\":" + s.stackSize + ",\"damage\":" + s.getItemDamage()
            + ",\"animationsToGo\":" + s.animationsToGo + "}";
    }
}
