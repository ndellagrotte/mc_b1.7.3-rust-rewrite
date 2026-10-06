package harness;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeSet;
import net.minecraft.src.Chunk;
import net.minecraft.src.Entity;
import net.minecraft.src.EntityArrow;
import net.minecraft.src.EntityItem;
import net.minecraft.src.EntityLiving;
import net.minecraft.src.EntityPlayer;
import net.minecraft.src.EnumSkyBlock;
import net.minecraft.src.InventoryPlayer;
import net.minecraft.src.ItemStack;
import net.minecraft.src.NextTickListEntry;
import net.minecraft.src.World;

/**
 * Tier B/C dump kinds (tests/golden/README.md "Dumps"). Dumps never load chunks: a dump that
 * reads an unloaded chunk is a scenario error. Every float is a bit pattern.
 */
final class Dumps {
    private Dumps() {}

    static Json.Line line(long t, String e) {
        return new Json.Line().put("t", t).put("e", e);
    }

    static boolean chunkLoaded(World w, int cx, int cz) {
        return w.getIChunkProvider().chunkExists(cx, cz);
    }

    static void requireBlockLoaded(World w, int x, int y, int z, String what) {
        if (y < 0 || y >= 128) throw new IllegalArgumentException(what + ": y " + y + " outside 0..127");
        if (!chunkLoaded(w, x >> 4, z >> 4)) {
            throw new IllegalStateException(what + " at " + x + "," + y + "," + z + ": chunk not loaded (dumps never load chunks)");
        }
    }

    // ---- world_time, rng_state ------------------------------------------------

    /** {@code World.getWorldTime} (World.java:2282). */
    static void worldTime(Trace t, long tick, World w) {
        t.add(line(tick, "world_time").put("time", w.getWorldTime()));
    }

    /**
     * World.rand's 48-bit state (World.java:37), the random-tick LCG {@code field_9437_g}
     * (World.java:25) and the HarnessMath (Math.random) stream's state.
     */
    static void rngState(Trace t, long tick, World w) {
        t.add(line(tick, "rng_state").put("world_rand", Rng.state(w.rand))
                .put("lcg", Reflect.getInt(World.class, "field_9437_g", w))
                .put("math_random", HarnessMath.state()));
    }

    // ---- block, blocks_full ---------------------------------------------------

    /**
     * {@code getBlockId} (World.java:311), {@code getBlockMetadata} (World.java:405) and
     * {@code getSavedLightValue} for Sky and Block (World.java:646-667).
     */
    static void block(Trace t, long tick, World w, int x, int y, int z) {
        requireBlockLoaded(w, x, y, z, "block dump");
        t.add(line(tick, "block").put("x", x).put("y", y).put("z", z)
                .put("id", w.getBlockId(x, y, z)).put("meta", w.getBlockMetadata(x, y, z))
                .put("skylight", w.getSavedLightValue(EnumSkyBlock.Sky, x, y, z))
                .put("blocklight", w.getSavedLightValue(EnumSkyBlock.Block, x, y, z)));
    }

    /** One byte per block, x outer, z middle, y inner (Chunk.blocks order, Chunk.java:239). */
    static void blockBox(Trace t, long tick, World w, int x0, int y0, int z0, int x1, int y1, int z1) {
        if (x1 < x0 || y1 < y0 || z1 < z0) throw new IllegalArgumentException("block_box: max < min");
        int n = (x1 - x0 + 1) * (y1 - y0 + 1) * (z1 - z0 + 1);
        if (n > 262144) throw new IllegalArgumentException("block_box larger than 262144 blocks");
        byte[] ids = new byte[n], metas = new byte[n], sky = new byte[n], light = new byte[n];
        int i = 0;
        for (int x = x0; x <= x1; x++) {
            for (int z = z0; z <= z1; z++) {
                for (int y = y0; y <= y1; y++) {
                    requireBlockLoaded(w, x, y, z, "block_box");
                    ids[i] = (byte) w.getBlockId(x, y, z);
                    metas[i] = (byte) w.getBlockMetadata(x, y, z);
                    sky[i] = (byte) w.getSavedLightValue(EnumSkyBlock.Sky, x, y, z);
                    light[i] = (byte) w.getSavedLightValue(EnumSkyBlock.Block, x, y, z);
                    i++;
                }
            }
        }
        t.add(line(tick, "blocks_full").put("x0", x0).put("y0", y0).put("z0", z0)
                .put("x1", x1).put("y1", y1).put("z1", z1)
                .put("ids", Hex.of(ids)).put("metas", Hex.of(metas))
                .put("skylight", Hex.of(sky)).put("blocklight", Hex.of(light)));
    }

    // ---- chunk_digest ---------------------------------------------------------

    static void chunkDigest(Trace t, long tick, World w, int cx, int cz, boolean full) {
        if (!chunkLoaded(w, cx, cz)) throw new IllegalStateException("chunk_digest " + cx + "," + cz + ": chunk not loaded");
        Chunk c = w.getChunkFromChunkCoords(cx, cz);
        TierA.chunkDigest(t, tick, c, cx, cz, "tick", full);
    }

    // ---- scheduled_ticks ------------------------------------------------------

    /**
     * {@code World.scheduledTickTreeSet} (World.java:17) in TreeSet order, i.e.
     * {@code NextTickListEntry.comparer}: scheduledTime, then tickEntryID (creation order;
     * NextTickListEntry.java:37-47). Per entry 24 bytes: xCoord, yCoord, zCoord, blockID as i32 LE,
     * scheduledTime as i64 LE (NextTickListEntry.java:5-9). tickEntryID is not dumped: it is a
     * JVM-global counter; its effect is the order.
     */
    @SuppressWarnings("unchecked")
    static void scheduledTicks(Trace t, long tick, World w, boolean full) {
        TreeSet<NextTickListEntry> set = (TreeSet<NextTickListEntry>) Reflect.get(World.class, "scheduledTickTreeSet", w);
        ByteBuffer buf = ByteBuffer.allocate(set.size() * 24).order(ByteOrder.LITTLE_ENDIAN);
        StringBuilder entries = new StringBuilder("[");
        for (NextTickListEntry e : set) {
            buf.putInt(e.xCoord).putInt(e.yCoord).putInt(e.zCoord).putInt(e.blockID).putLong(e.scheduledTime);
            if (entries.length() > 1) entries.append(',');
            entries.append('[').append(e.xCoord).append(',').append(e.yCoord).append(',').append(e.zCoord)
                    .append(',').append(e.blockID).append(',').append(e.scheduledTime).append(']');
        }
        Json.Line l = line(tick, "scheduled_ticks").put("count", set.size()).put("blake3", Blake3.hex(buf.array()));
        if (full) l.raw("entries", entries.append(']').toString());
        t.add(l);
    }

    // ---- entity_state ---------------------------------------------------------

    /** {@code which}: "all" or a list of entity ids. Lines in loadedEntityList order (World.java:15). */
    @SuppressWarnings("unchecked")
    static void entityState(Trace t, long tick, World w, Object which) {
        List<Entity> list = (List<Entity>) w.loadedEntityList;
        if ("all".equals(which)) {
            for (int i = 0; i < list.size(); i++) t.add(entity(tick, i, list.get(i)));
            return;
        }
        if (!(which instanceof List)) throw new IllegalArgumentException("entity_state \"which\" must be \"all\" or a list of ids");
        List<Long> ids = new ArrayList<>();
        for (Object o : (List<Object>) which) ids.add((Long) o);
        List<Long> seen = new ArrayList<>();
        for (int i = 0; i < list.size(); i++) {
            Entity e = list.get(i);
            if (ids.contains((long) e.entityId)) {
                t.add(entity(tick, i, e));
                seen.add((long) e.entityId);
            }
        }
        for (Long id : ids) {
            if (!seen.contains(id)) t.add(line(tick, "entity_absent").put("id", id));
        }
    }

    static Json.Line entity(long tick, int idx, Entity e) {
        Json.Line l = line(tick, "entity").put("idx", idx).put("id", e.entityId).put("kind", e.getClass().getSimpleName())
                .f64("posX", e.posX).f64("posY", e.posY).f64("posZ", e.posZ)
                .f64("prevPosX", e.prevPosX).f64("prevPosY", e.prevPosY).f64("prevPosZ", e.prevPosZ)
                .f64("motionX", e.motionX).f64("motionY", e.motionY).f64("motionZ", e.motionZ)
                .f32("rotationYaw", e.rotationYaw).f32("rotationPitch", e.rotationPitch)
                .put("onGround", e.onGround)
                .put("isCollidedHorizontally", e.isCollidedHorizontally)
                .put("isCollidedVertically", e.isCollidedVertically)
                .f32("fallDistance", Reflect.getFloat(Entity.class, "fallDistance", e))
                .put("isDead", e.isDead).put("ticksExisted", e.ticksExisted)
                .put("rand", Rng.state(HarnessWorld.entityRand(e)));
        Json.Line s = new Json.Line();
        if (e instanceof EntityLiving) {
            EntityLiving v = (EntityLiving) e;
            s.put("health", v.health).put("attackTime", v.attackTime).put("hurtTime", v.hurtTime)
                    .put("deathTime", v.deathTime)
                    .f32("moveForward", Reflect.getFloat(EntityLiving.class, "moveForward", v))
                    .f32("moveStrafing", Reflect.getFloat(EntityLiving.class, "moveStrafing", v))
                    .put("isJumping", Reflect.getBoolean(EntityLiving.class, "isJumping", v));
        }
        if (e instanceof EntityPlayer) {
            InventoryPlayer inv = ((EntityPlayer) e).inventory;
            s.put("currentItem", inv.currentItem).put("inventory", Blake3.hex(inventoryBytes(inv)));
        }
        if (e instanceof EntityItem) {
            EntityItem it = (EntityItem) e;
            s.raw("item", stack(it.item)).put("age", it.age).put("delayBeforeCanPickup", it.delayBeforeCanPickup)
                    .put("health", Reflect.getInt(EntityItem.class, "health", it));
        }
        if (e instanceof EntityArrow) {
            EntityArrow a = (EntityArrow) e;
            s.put("inGround", Reflect.getBoolean(EntityArrow.class, "inGround", a))
                    .put("ticksInAir", Reflect.getInt(EntityArrow.class, "ticksInAir", a))
                    .put("arrowShake", a.arrowShake);
        }
        return l.raw("state", s.toString());
    }

    /** {@code {"id":..,"count":..,"damage":..}} or {@code null}. */
    static String stack(ItemStack s) {
        if (s == null) return "null";
        return new Json.Line().put("id", s.itemID).put("count", s.stackSize).put("damage", s.getItemDamage()).toString();
    }

    /**
     * mainInventory[0..35] then armorInventory[0..3] (InventoryPlayer.java:4-5), 12 bytes each:
     * itemID, stackSize, itemDamage as i32 LE; an empty slot is (-1, 0, 0).
     */
    static byte[] inventoryBytes(InventoryPlayer inv) {
        ByteBuffer buf = ByteBuffer.allocate(40 * 12).order(ByteOrder.LITTLE_ENDIAN);
        for (ItemStack[] part : new ItemStack[][] {inv.mainInventory, inv.armorInventory}) {
            for (ItemStack s : part) {
                if (s == null) buf.putInt(-1).putInt(0).putInt(0);
                else buf.putInt(s.itemID).putInt(s.stackSize).putInt(s.getItemDamage());
            }
        }
        return buf.array();
    }

    static Map<String, Object> map(Object o, String what) {
        if (!(o instanceof Map)) throw new IllegalArgumentException(what + " must be an object");
        @SuppressWarnings("unchecked") Map<String, Object> m = (Map<String, Object>) o;
        return m;
    }
}
