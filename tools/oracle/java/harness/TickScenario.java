package harness;

import static harness.Main.Scenario.list;
import static harness.Main.Scenario.num;
import static harness.Main.Scenario.seed;
import static harness.Main.Scenario.str;

import java.lang.reflect.Constructor;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import net.minecraft.src.AxisAlignedBB;
import net.minecraft.src.Entity;
import net.minecraft.src.EntityItem;
import net.minecraft.src.EntityList;
import net.minecraft.src.EntityPlayer;
import net.minecraft.src.EntitySlime;
import net.minecraft.src.InventoryPlayer;
import net.minecraft.src.ItemStack;
import net.minecraft.src.MathHelper;
import net.minecraft.src.PlayerControllerSP;
import net.minecraft.src.Vec3D;
import net.minecraft.src.World;

/**
 * Scenario kind {@code tick} (tiers B, C, S): a vanilla world ticked under a script of ops and
 * dumps (tests/golden/README.md "Tier B/C/S scenarios").
 */
final class TickScenario {
    private TickScenario() {}

    private static final List<String> OPS = List.of("set_block", "set_block_silent", "fill", "set_time", "preload",
            "player", "spawn", "set_motion", "set_pos", "set_rotation", "use_item", "attack", "save");
    private static final List<String> DUMPS = List.of("chunk_digest", "block", "block_box", "scheduled_ticks",
            "entity_state", "world_time", "rng_state", "ray_trace", "power");

    @SuppressWarnings("unchecked")
    static Trace run(Map<String, Object> sc, Path dir) throws Exception {
        long seed = seed(sc);
        String dimension = str(sc, "dimension");
        int ticks = Math.toIntExact(num(sc, "ticks"));
        if (ticks < 1) throw new IllegalArgumentException("\"ticks\" must be >= 1");
        String worldType = sc.containsKey("world") ? str(sc, "world") : "generated";
        String saveFormat = sc.containsKey("save_format") ? str(sc, "save_format") : "none";
        // Default 0 (peaceful), not GameSettings' 2: hostile spawn checks draw from the unseeded
        // Entity.rand before the entity joins the world (EntityMob.java:69-85, EntityGhast.java:151-153,
        // EntitySlime.java:134-137, called at SpawnerAnimals.java:130), so they are not reproducible.
        int difficulty = sc.containsKey("difficulty") ? Math.toIntExact(num(sc, "difficulty")) : 0;
        if (difficulty < 0 || difficulty > 3) throw new IllegalArgumentException("difficulty must be 0..3");
        String mobSpawning = sc.containsKey("mob_spawning") ? str(sc, "mob_spawning") : "default";
        if (!mobSpawning.equals("default") && !mobSpawning.equals("none")) {
            throw new IllegalArgumentException("mob_spawning must be \"default\" or \"none\"");
        }

        List<Map<String, Object>> ops = new ArrayList<>();
        for (Object o : list(sc, "ops")) {
            Map<String, Object> op = Dumps.map(o, "op");
            long t = num(op, "t");
            if (t < 0 || t >= ticks) throw new IllegalArgumentException("op t=" + t + " outside 0.." + (ticks - 1));
            if (!OPS.contains(str(op, "op"))) throw new IllegalArgumentException("unknown op " + op.get("op"));
            if (str(op, "op").equals("save") && !saveFormat.equals("mcregion")) {
                throw new IllegalArgumentException("op save needs \"save_format\": \"mcregion\"");
            }
            ops.add(op);
        }
        if (difficulty > 0 && !mobSpawning.equals("none") && ops.stream().anyMatch(o -> "player".equals(o.get("op")))) {
            System.err.println("harness: warning: " + sc.get("name") + ": difficulty " + difficulty
                    + " with a player enables hostile mob spawning, which is not reproducible (tools/oracle/census.md)");
        }
        List<Map<String, Object>> dumps = new ArrayList<>();
        for (Object o : list(sc, "dumps")) {
            Map<String, Object> d = Dumps.map(o, "dump");
            if (!DUMPS.contains(str(d, "what"))) throw new IllegalArgumentException("unknown dump " + d.get("what"));
            if (d.containsKey("every") == d.containsKey("t")) {
                throw new IllegalArgumentException("dump needs exactly one of \"every\" and \"t\"");
            }
            if (d.containsKey("every") && num(d, "every") < 1) throw new IllegalArgumentException("\"every\" must be >= 1");
            dumps.add(d);
        }

        // Every vanilla Math.random() draws from HarnessMath: one stream per scenario.
        HarnessMath.seed(seed);
        HarnessWorld hw = HarnessWorld.create(seed, dimension, worldType, saveFormat, difficulty);
        hw.noMobSpawning = mobSpawning.equals("none");
        Trace trace = new Trace();
        try {
            implicitPreload(hw, ops, trace);
            int next = 0;
            // Ops grouped by t, file order within a tick.
            List<Map<String, Object>> sorted = new ArrayList<>(ops);
            sorted.sort((a, b) -> Long.compare(num(a, "t"), num(b, "t")));
            for (int t = 0; t < ticks; t++) {
                // Frame start, Minecraft.java:519-520 (one frame per tick).
                AxisAlignedBB.clearBoundingBoxPool();
                Vec3D.initialize();
                while (next < sorted.size() && num(sorted.get(next), "t") == t) {
                    apply(hw, sorted.get(next), t, trace);
                    next++;
                }
                hw.runTick();
                // Once per frame after the tick loop, Minecraft.java:553 (at most 500 updates).
                hw.world.updatingLighting();
                for (Map<String, Object> d : dumps) {
                    boolean due = d.containsKey("every") ? (t + 1) % num(d, "every") == 0 : num(d, "t") == t;
                    if (due) dump(hw, d, t, trace);
                }
            }
        } finally {
            hw.close();
        }
        return trace;
    }

    // ---- preload ----------------------------------------------------------------

    /**
     * Before tick 0: chunks within 32 blocks (World.updateEntityWithOptionalForce needs
     * checkChunksExist(x-32..x+32, z-32..z+32) to tick an entity, World.java:1291-1296) of every
     * position named by a set_block, set_block_silent, fill (both corners), player, spawn,
     * set_pos or use_item op, in op file order; per position the 5x5 chunk square
     * (floor(x)>>4 - 2 .. +2), dz outer, dx inner.
     */
    private static void implicitPreload(HarnessWorld hw, List<Map<String, Object>> ops, Trace trace) {
        int loaded = 0;
        for (Map<String, Object> op : ops) {
            for (double[] p : positions(op)) {
                int cx = MathHelper.floor_double(p[0]) >> 4, cz = MathHelper.floor_double(p[1]) >> 4;
                loaded += preloadSquare(hw.world, cx, cz, 2);
            }
        }
        trace.add(Dumps.line(0, "preload").put("implicit", true).put("loaded", loaded));
    }

    /** [x, z] pairs an op names. */
    private static List<double[]> positions(Map<String, Object> op) {
        List<double[]> out = new ArrayList<>();
        switch (str(op, "op")) {
            case "set_block": case "set_block_silent": case "player": case "use_item":
                out.add(new double[] {Num.f64(op.get("x")), Num.f64(op.get("z"))});
                break;
            case "fill":
                out.add(new double[] {num(op, "x0"), num(op, "z0")});
                out.add(new double[] {num(op, "x1"), num(op, "z1")});
                break;
            case "spawn": case "set_pos": {
                double[] v = Num.vec3(op.get("pos"));
                out.add(new double[] {v[0], v[2]});
                break;
            }
            default: break;
        }
        return out;
    }

    /**
     * {@code getChunkFromChunkCoords} over the square, dz outer, dx inner; after each newly loaded
     * chunk the light queue is drained, as Minecraft's world start does (Minecraft.java:1366-1373).
     */
    private static int preloadSquare(World w, int cx, int cz, int r) {
        int loaded = 0;
        for (int dz = -r; dz <= r; dz++) {
            for (int dx = -r; dx <= r; dx++) {
                if (!Dumps.chunkLoaded(w, cx + dx, cz + dz)) loaded++;
                w.getChunkFromChunkCoords(cx + dx, cz + dz);
                while (w.updatingLighting()) {
                    // drain, Minecraft.java:1370
                }
            }
        }
        return loaded;
    }

    // ---- ops --------------------------------------------------------------------

    private static void apply(HarnessWorld hw, Map<String, Object> op, int t, Trace trace) throws Exception {
        World w = hw.world;
        String name = str(op, "op");
        Json.Line l = Dumps.line(t, "op").put("op", name);
        switch (name) {
            case "set_block": {
                // src: World.java:461-468 (Chunk.setBlockIDWithMetadata, which calls the new block's
                // onBlockAdded, Chunk.java:236-273; then notifyBlockChange: neighbours'
                // onNeighborBlockChange, World.java:477-480, :506-523)
                int x = i(op, "x"), y = i(op, "y"), z = i(op, "z");
                l.put("ok", w.setBlockAndMetadataWithNotify(x, y, z, i(op, "id"), meta(op)));
                break;
            }
            case "set_block_silent": {
                // src: World.java:370-384, no neighbour notification (onBlockAdded still runs)
                l.put("ok", w.setBlockAndMetadata(i(op, "x"), i(op, "y"), i(op, "z"), i(op, "id"), meta(op)));
                break;
            }
            case "fill": {
                int x0 = i(op, "x0"), y0 = i(op, "y0"), z0 = i(op, "z0"), x1 = i(op, "x1"), y1 = i(op, "y1"), z1 = i(op, "z1");
                if (x1 < x0 || y1 < y0 || z1 < z0) throw new IllegalArgumentException("fill: max < min");
                int id = i(op, "id"), meta = meta(op), changed = 0;
                for (int x = x0; x <= x1; x++) {
                    for (int z = z0; z <= z1; z++) {
                        for (int y = y0; y <= y1; y++) {
                            if (w.setBlockAndMetadata(x, y, z, id, meta)) changed++;
                        }
                    }
                }
                l.put("changed", changed);
                break;
            }
            case "set_time":
                w.setWorldTime(num(op, "time")); // src: World.java:2274-2276
                break;
            case "preload":
                l.put("loaded", preloadSquare(w, i(op, "cx"), i(op, "cz"), i(op, "radius")));
                break;
            case "player": {
                if (hw.player != null) throw new IllegalArgumentException("only one player op");
                HarnessPlayer p = new HarnessPlayer(w);
                seedAfterConstruction(p, hw.seed);
                place(p, op);
                boolean joined = w.entityJoinedWorld(p); // src: World.java:896-918
                hw.player = p;
                l.put("id", p.entityId).put("joined", joined);
                break;
            }
            case "spawn": {
                Entity e = construct(w, op, hw.seed);
                if (!str(op, "kind").equals("Item")) place(e, op);
                if (op.containsKey("id") && num(op, "id") != e.entityId) {
                    throw new IllegalStateException("spawn: entity got id " + e.entityId + ", scenario says " + num(op, "id"));
                }
                boolean joined = w.entityJoinedWorld(e); // src: World.java:896-918
                l.put("id", e.entityId).put("kind", e.getClass().getSimpleName()).put("joined", joined);
                break;
            }
            case "set_motion": {
                Entity e = entity(w, op);
                double[] v = Num.vec3(op.get("motion"));
                e.motionX = v[0];
                e.motionY = v[1];
                e.motionZ = v[2];
                l.put("id", e.entityId);
                break;
            }
            case "set_pos": {
                Entity e = entity(w, op);
                double[] v = Num.vec3(op.get("pos"));
                e.setPosition(v[0], v[1], v[2]); // src: Entity.java:163-170
                l.put("id", e.entityId);
                break;
            }
            case "set_rotation": {
                Entity e = entity(w, op);
                e.rotationYaw = Num.f32(op.get("yaw"));
                e.rotationPitch = Num.f32(op.get("pitch"));
                l.put("id", e.entityId);
                break;
            }
            case "use_item":
                useItem(hw, op, l);
                break;
            case "attack":
                attack(hw, op, l);
                break;
            case "save":
                trace.add(l);
                TierS.save(hw, op, t, trace);
                return;
            default: throw new IllegalArgumentException("unknown op " + name);
        }
        trace.add(l);
    }

    private static int i(Map<String, Object> m, String k) {
        return Math.toIntExact(num(m, k));
    }

    private static int meta(Map<String, Object> op) {
        return op.containsKey("meta") ? i(op, "meta") : 0;
    }

    /**
     * {@code setLocationAndAngles} (Entity.java:669-676; posY = y + yOffset, so y is the bounding
     * box bottom). yaw/pitch default to the entity's current values (EntityLiving's constructor
     * draws a random yaw, EntityLiving.java:66).
     */
    private static void place(Entity e, Map<String, Object> op) {
        double x, y, z;
        if (op.containsKey("pos")) {
            double[] v = Num.vec3(op.get("pos"));
            x = v[0];
            y = v[1];
            z = v[2];
        } else {
            x = Num.f64(op.get("x"));
            y = Num.f64(op.get("y"));
            z = Num.f64(op.get("z"));
        }
        float yaw = op.containsKey("yaw") ? Num.f32(op.get("yaw")) : e.rotationYaw;
        float pitch = op.containsKey("pitch") ? Num.f32(op.get("pitch")) : e.rotationPitch;
        e.setLocationAndAngles(x, y, z, yaw, pitch);
    }

    /**
     * Constructs {@code kind} with its {@code (World)} constructor, the class taken from
     * EntityList's name table (EntityList.java:7, filled at :82-107). {@code "Item"} uses
     * {@code EntityItem(World, x, y, z, ItemStack)} (EntityItem.java:11-21, as block drops do,
     * Block.java:355-364) with {@code delayBeforeCanPickup = pickup_delay} (default 10, Block.java:362).
     */
    private static Entity construct(World w, Map<String, Object> op, long seed) throws Exception {
        String kind = str(op, "kind");
        Entity e;
        if (kind.equals("Item")) {
            Map<String, Object> it = Dumps.map(op.get("item"), "spawn Item \"item\"");
            double[] p = Num.vec3(op.get("pos"));
            ItemStack stack = new ItemStack(i(it, "id"), i(it, "count"), it.containsKey("damage") ? i(it, "damage") : 0);
            EntityItem item = new EntityItem(w, p[0], p[1], p[2], stack);
            item.delayBeforeCanPickup = op.containsKey("pickup_delay") ? i(op, "pickup_delay") : 10;
            e = item;
        } else {
            @SuppressWarnings("unchecked")
            Map<String, Class<?>> names = (Map<String, Class<?>>) Reflect.get(EntityList.class, "stringToClassMapping", null);
            Class<?> c = names.get(kind);
            if (c == null) throw new IllegalArgumentException("unknown entity kind " + kind + " (EntityList names)");
            Constructor<?> k = c.getConstructor(World.class);
            e = (Entity) k.newInstance(w);
        }
        seedAfterConstruction(e, seed);
        return e;
    }

    /**
     * {@code rand.setSeed(mix(seed, id))} right after construction. EntitySlime's constructor
     * draws its size and jump delay from the then unseeded {@code new Random()} (Entity.java:96,
     * EntitySlime.java:11-14); both are replayed on the seeded stream (the size is persisted in
     * NBT, so the join hook cannot redraw it). Chicken and squid constructor draws are redrawn by
     * the join hook (HarnessWorld.seedOnJoin).
     */
    static void seedAfterConstruction(Entity e, long seed) {
        HarnessWorld.seedEntity(e, seed);
        if (e instanceof EntitySlime) {
            Random r = HarnessWorld.entityRand(e);
            int size = 1 << r.nextInt(3);
            Reflect.set(EntitySlime.class, "slimeJumpDelay", e, r.nextInt(20) + 10);
            ((EntitySlime) e).setSlimeSize(size);
        }
    }

    @SuppressWarnings("unchecked")
    private static Entity entity(World w, Map<String, Object> op) {
        long id = num(op, "id");
        for (Entity e : (List<Entity>) w.loadedEntityList) if (e.entityId == id) return e;
        throw new IllegalStateException(op.get("op") + ": no entity with id " + id + " in loadedEntityList");
    }

    /**
     * Right click on a block: Minecraft.clickMouse(1), TILE branch (Minecraft.java:819-850), with
     * the hit given by the op instead of a ray trace. The item stack goes into hotbar slot
     * {@code inventory.currentItem} first. PlayerControllerSP does not override sendPlaceBlock or
     * sendUseItem (PlayerController.java:47-61, :75-82), neither touches {@code mc}.
     */
    private static void useItem(HarnessWorld hw, Map<String, Object> op, Json.Line l) {
        EntityPlayer p = hw.player;
        if (p == null) throw new IllegalArgumentException("use_item needs a player op first");
        InventoryPlayer inv = p.inventory;
        if (op.containsKey("item_id")) {
            inv.mainInventory[inv.currentItem] = new ItemStack(i(op, "item_id"),
                    op.containsKey("count") ? i(op, "count") : 1, meta(op));
        }
        PlayerControllerSP pc = new PlayerControllerSP(null);
        World w = hw.world;
        int x = i(op, "x"), y = i(op, "y"), z = i(op, "z"), side = i(op, "side");
        boolean notPlaced = true;
        ItemStack held = inv.getCurrentItem();
        boolean placed = pc.sendPlaceBlock(p, w, held, x, y, z, side);
        if (placed) {
            notPlaced = false;
            p.swingItem();
        }
        boolean used = false;
        if (held != null) {
            if (held.stackSize == 0) inv.mainInventory[inv.currentItem] = null;
            if (notPlaced) {
                ItemStack again = inv.getCurrentItem();
                used = again != null && pc.sendUseItem(p, w, again);
            }
        }
        l.put("placed", placed).put("used", used).raw("stack", Dumps.stack(inv.getCurrentItem()));
    }

    /**
     * Attacker = the player: Minecraft.clickMouse(0), ENTITY branch (Minecraft.java:802-814):
     * swingItem, then PlayerController.attackEntity = attackTargetEntityWithCurrentItem
     * (PlayerController.java:92-94, EntityPlayer.java:488). Otherwise
     * {@code target.attackEntityFrom(attacker or null, damage)} (Entity.java:746,
     * EntityLiving.java:307).
     */
    private static void attack(HarnessWorld hw, Map<String, Object> op, Json.Line l) {
        Map<String, Object> t = new LinkedHashMap<>(op);
        t.put("id", op.get("target_id"));
        Entity target = entity(hw.world, t);
        Entity attacker = null;
        if (op.containsKey("attacker_id")) {
            Map<String, Object> a = new LinkedHashMap<>(op);
            a.put("id", op.get("attacker_id"));
            attacker = entity(hw.world, a);
        }
        l.put("target_id", target.entityId);
        if (attacker != null && attacker == hw.player) {
            hw.player.swingItem();
            hw.player.attackTargetEntityWithCurrentItem(target);
            l.put("via", "player");
        } else {
            l.put("via", "direct").put("result", target.attackEntityFrom(attacker, i(op, "damage")));
        }
    }

    // ---- dumps ------------------------------------------------------------------

    private static void dump(HarnessWorld hw, Map<String, Object> d, int t, Trace trace) {
        World w = hw.world;
        switch (str(d, "what")) {
            case "chunk_digest": Dumps.chunkDigest(trace, t, w, i(d, "x"), i(d, "z"), Main.Scenario.flag(d, "full")); break;
            case "block": Dumps.block(trace, t, w, i(d, "x"), i(d, "y"), i(d, "z")); break;
            case "block_box":
                Dumps.blockBox(trace, t, w, i(d, "x0"), i(d, "y0"), i(d, "z0"), i(d, "x1"), i(d, "y1"), i(d, "z1"));
                break;
            case "scheduled_ticks": Dumps.scheduledTicks(trace, t, w, Main.Scenario.flag(d, "full")); break;
            case "entity_state": Dumps.entityState(trace, t, w, d.get("which")); break;
            case "world_time": Dumps.worldTime(trace, t, w); break;
            case "rng_state": Dumps.rngState(trace, t, w); break;
            case "ray_trace":
                Dumps.rayTrace(trace, t, w, Num.vec3(d.get("from")), Num.vec3(d.get("to")), Main.Scenario.flag(d, "stop_on_liquid"));
                break;
            case "power": Dumps.power(trace, t, w, i(d, "x"), i(d, "y"), i(d, "z")); break;
            default: throw new IllegalArgumentException("unknown dump " + d.get("what"));
        }
    }
}
