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
import net.minecraft.src.EntityEgg;
import net.minecraft.src.EntityFireball;
import net.minecraft.src.EntityFish;
import net.minecraft.src.EntityItem;
import net.minecraft.src.EntityLightningBolt;
import net.minecraft.src.EntityList;
import net.minecraft.src.EntityLiving;
import net.minecraft.src.EntityMinecart;
import net.minecraft.src.EntityPainting;
import net.minecraft.src.EntityPlayer;
import net.minecraft.src.EntitySlime;
import net.minecraft.src.EnumArt;
import net.minecraft.src.Explosion;
import net.minecraft.src.InventoryPlayer;
import net.minecraft.src.ItemStack;
import net.minecraft.src.MathHelper;
import net.minecraft.src.PlayerControllerSP;
import net.minecraft.src.Vec3D;
import net.minecraft.src.World;
import net.minecraft.src.WorldInfo;

/**
 * Scenario kind {@code tick} (tiers B, C, S): a vanilla world ticked under a script of ops and
 * dumps (tests/golden/README.md "Tier B/C/S scenarios").
 */
final class TickScenario {
    private TickScenario() {}

    private static final List<String> OPS = List.of("set_block", "set_block_silent", "fill", "set_time", "set_weather", "preload",
            "player", "spawn", "set_motion", "set_pos", "set_rotation", "use_item", "attack", "save", "explode",
            "interact");
    private static final List<String> DUMPS = List.of("chunk_digest", "block", "block_box", "scheduled_ticks",
            "entity_state", "world_time", "rng_state", "ray_trace", "power", "weather");

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
            case "set_weather": {
                // src: WorldInfo.java:195-221 (plain setters), each only when given
                WorldInfo info = w.getWorldInfo();
                if (op.containsKey("rain_time")) info.setRainTime(i(op, "rain_time"));
                if (op.containsKey("raining")) info.setRaining(bool(op, "raining"));
                if (op.containsKey("thunder_time")) info.setThunderTime(i(op, "thunder_time"));
                if (op.containsKey("thundering")) info.setThundering(bool(op, "thundering"));
                break;
            }
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
                Entity e = construct(hw, op, hw.seed);
                if (placedByOp(op)) place(e, op);
                if (op.containsKey("id") && num(op, "id") != e.entityId) {
                    throw new IllegalStateException("spawn: entity got id " + e.entityId + ", scenario says " + num(op, "id"));
                }
                boolean joined = str(op, "kind").equals("LightningBolt")
                        ? w.addWeatherEffect(e) // src: World.java:891-894
                        : w.entityJoinedWorld(e); // src: World.java:896-918
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
            case "interact":
                interact(hw, op, l);
                break;
            case "explode":
                explode(hw, op, l);
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

    private static boolean bool(Map<String, Object> m, String k) {
        Object v = m.get(k);
        if (!(v instanceof Boolean)) throw new IllegalArgumentException("\"" + k + "\" must be true or false");
        return (Boolean) v;
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
     * Whether {@code place} runs after construction: not for the kinds whose constructor takes
     * the position ({@code Item}, {@code LightningBolt}, {@code FishingHook}, a {@code Painting}
     * with a {@code direction}, a {@code Minecart} with a {@code type}).
     */
    private static boolean placedByOp(Map<String, Object> op) {
        switch (str(op, "kind")) {
            case "Item": case "LightningBolt": case "FishingHook": return false;
            case "Painting": return !op.containsKey("direction");
            case "Minecart": return !op.containsKey("type");
            default: return true;
        }
    }

    /**
     * Constructs {@code kind} with its {@code (World)} constructor, the class taken from
     * EntityList's name table (EntityList.java:7, filled at :82-107). {@code "Item"} uses
     * {@code EntityItem(World, x, y, z, ItemStack)} (EntityItem.java:11-21, as block drops do,
     * Block.java:355-364) with {@code delayBeforeCanPickup = pickup_delay} (default 10, Block.java:362).
     * {@code "Painting"} with {@code direction} uses {@code EntityPainting(World, floor(x),
     * floor(y), floor(z), direction)} (EntityPainting.java:22-45, as ItemPainting.java:27 does).
     * Harness-defined kinds, not in EntityList's table: {@code "Egg"} ({@code EntityEgg(World)},
     * EntityEgg.java:16-19), {@code "Fireball"} ({@code EntityFireball(World)},
     * EntityFireball.java:19-22), {@code "LightningBolt"} ({@code EntityLightningBolt(World, x, y,
     * z)}, EntityLightningBolt.java:10-34, joined with {@code addWeatherEffect} as World.java:1927
     * does) and {@code "FishingHook"} ({@code EntityFish(World, player)}, EntityFish.java:48-74,
     * as ItemFishingRod.java:26 does; sets {@code player.fishEntity}). {@code "Minecart"} with
     * {@code type} (0 plain, 1 chest, 2 furnace) uses {@code EntityMinecart(World, x, y, z, type)}
     * (EntityMinecart.java:57-67, as ItemMinecart.java:16 does; no draws).
     */
    private static Entity construct(HarnessWorld hw, Map<String, Object> op, long seed) throws Exception {
        World w = hw.world;
        String kind = str(op, "kind");
        Entity e;
        if (kind.equals("Egg")) {
            e = new EntityEgg(w);
        } else if (kind.equals("Fireball")) {
            e = new EntityFireball(w);
        } else if (kind.equals("LightningBolt")) {
            return lightningBolt(hw, op, seed);
        } else if (kind.equals("FishingHook")) {
            return fishingHook(hw, seed);
        } else if (kind.equals("Painting") && op.containsKey("direction")) {
            return painting(w, op, seed);
        } else if (kind.equals("Minecart") && op.containsKey("type")) {
            double[] p = Num.vec3(op.get("pos"));
            e = new EntityMinecart(w, p[0], p[1], p[2], i(op, "type"));
        } else if (kind.equals("Item")) {
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

    /**
     * {@code new EntityLightningBolt(world, x, y, z)}. Its constructor draws {@code boltVertex =
     * rand.nextLong()} and {@code boltLivingTime = rand.nextInt(3) + 1} (EntityLightningBolt.java:14-15)
     * from the unseeded {@code new Random()} (Entity.java:96), and a weather effect never passes
     * the join hook (census §3): both are replayed on the seeded stream, as the slime's size is.
     * At difficulty 2 and more the constructor also places fire at rand-jittered blocks
     * (:16-32), which no replay can undo: refused.
     */
    private static Entity lightningBolt(HarnessWorld hw, Map<String, Object> op, long seed) {
        if (hw.difficulty >= 2) {
            throw new IllegalArgumentException("spawn LightningBolt needs difficulty < 2 (its constructor places "
                    + "fire with the unseeded rand, census §3)");
        }
        double[] p = Num.vec3(op.get("pos"));
        EntityLightningBolt b = new EntityLightningBolt(hw.world, p[0], p[1], p[2]);
        seedAfterConstruction(b, seed);
        Random r = HarnessWorld.entityRand(b);
        b.boltVertex = r.nextLong();
        Reflect.set(EntityLightningBolt.class, "boltLivingTime", b, r.nextInt(3) + 1);
        return b;
    }

    /**
     * {@code new EntityFish(world, player)} (the player op's player): its constructor ends with
     * {@code calculateVelocity}, three {@code nextGaussian} draws on the unseeded rand
     * (EntityFish.java:73, :90-92, census §3). Replayed on the seeded stream: lines 69-73 again
     * from the angler's yaw and pitch, which the constructor's {@code setLocationAndAngles}
     * copied unchanged (:63); the draws only reach motion and rotation, so the replay leaves
     * the entity as a constructor with a seeded rand would.
     */
    private static Entity fishingHook(HarnessWorld hw, long seed) {
        if (hw.player == null) throw new IllegalArgumentException("spawn FishingHook needs a player op first");
        EntityFish f = new EntityFish(hw.world, hw.player);
        seedAfterConstruction(f, seed);
        float yaw = hw.player.rotationYaw, pitch = hw.player.rotationPitch, s = 0.4F;
        double mx = (double)(-MathHelper.sin(yaw / 180.0F * 3.1415927F) * MathHelper.cos(pitch / 180.0F * 3.1415927F) * s);
        double mz = (double)(MathHelper.cos(yaw / 180.0F * 3.1415927F) * MathHelper.cos(pitch / 180.0F * 3.1415927F) * s);
        double my = (double)(-MathHelper.sin(pitch / 180.0F * 3.1415927F) * s);
        f.calculateVelocity(mx, my, mz, 1.5F, 1.0F);
        return f;
    }

    /**
     * {@code new EntityPainting(world, floor(x), floor(y), floor(z), direction)}: its art choice
     * draws {@code rand.nextInt(fits)} on the unseeded rand (EntityPainting.java:41, census §3).
     * Replayed on the seeded stream: the constructor's loop (:27-44) again with the public
     * {@code art}, {@code setDirection} and {@code onValidSurface}; the painting is not in the
     * world yet, as during the constructor.
     */
    private static Entity painting(World w, Map<String, Object> op, long seed) {
        double[] p = Num.vec3(op.get("pos"));
        int x = MathHelper.floor_double(p[0]), y = MathHelper.floor_double(p[1]), z = MathHelper.floor_double(p[2]);
        int dir = i(op, "direction");
        EntityPainting e = new EntityPainting(w, x, y, z, dir);
        seedAfterConstruction(e, seed);
        List<EnumArt> fit = new ArrayList<>();
        for (EnumArt a : EnumArt.values()) {
            e.art = a;
            e.setDirection(dir);
            if (e.onValidSurface()) fit.add(a);
        }
        if (fit.size() > 0) e.art = fit.get(HarnessWorld.entityRand(e).nextInt(fit.size()));
        e.setDirection(dir);
        return e;
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

    /**
     * Right click on an entity as the player: Minecraft.clickMouse(1), ENTITY branch
     * (Minecraft.java:800-817), with the op's entity instead of the ray trace. The item stack (if
     * {@code item_id} is given) goes into hotbar slot {@code inventory.currentItem} first.
     * PlayerController.interactWithEntity (PlayerController.java:88-90) is
     * EntityPlayer.useCurrentItemOnEntity (EntityPlayer.java:457-469), inlined here to report
     * {@code target.interact(player)}: when that is false, the held stack's
     * {@code useItemOnEntity} (ItemStack.java:170-172) on a living target, and its removal at size
     * 0 or less. The entity branch leaves clickMouse's {@code var2} true, so
     * {@code sendUseItem} with the held stack follows (Minecraft.java:846-850).
     */
    private static void interact(HarnessWorld hw, Map<String, Object> op, Json.Line l) {
        EntityPlayer p = hw.player;
        if (p == null) throw new IllegalArgumentException("interact needs a player op first");
        Entity target = entity(hw.world, op);
        InventoryPlayer inv = p.inventory;
        if (op.containsKey("item_id")) {
            inv.mainInventory[inv.currentItem] = new ItemStack(i(op, "item_id"),
                    op.containsKey("count") ? i(op, "count") : 1, meta(op));
        }
        boolean result = target.interact(p);
        if (!result) {
            ItemStack s = p.getCurrentEquippedItem();
            if (s != null && target instanceof EntityLiving) {
                s.useItemOnEntity((EntityLiving) target);
                if (s.stackSize <= 0) {
                    s.onItemDestroyedByUse(p);
                    p.destroyCurrentEquippedItem();
                }
            }
        }
        ItemStack again = inv.getCurrentItem();
        boolean used = again != null && new PlayerControllerSP(null).sendUseItem(p, hw.world, again);
        l.put("id", target.entityId).put("result", result).put("used", used)
                .raw("stack", Dumps.stack(inv.getCurrentItem()));
    }

    /**
     * {@code World.createExplosion(null, x, y, z, size)} (World.java:1518-1520), or with
     * {@code "flaming": true} the body of {@code World.newExplosion(null, x, y, z, size, true)}
     * (World.java:1522-1528) with the explosion's {@code ExplosionRNG} (Explosion.java:11, a
     * clock-seeded {@code new Random()}, census §2) set to {@code mix(seed, -6)} before
     * {@code doExplosionA}, the only method that draws it (the fire roll, Explosion.java:118).
     * No exploder: nothing is excluded from the entity pass and the attacker is null.
     */
    private static void explode(HarnessWorld hw, Map<String, Object> op, Json.Line l) {
        World w = hw.world;
        double x = Num.f64(op.get("x")), y = Num.f64(op.get("y")), z = Num.f64(op.get("z"));
        float size = Num.f32(op.get("size"));
        boolean flaming = op.containsKey("flaming") && bool(op, "flaming");
        Explosion e;
        if (!flaming) {
            e = w.createExplosion(null, x, y, z, size);
        } else {
            e = new Explosion(w, null, x, y, z, size);
            e.isFlaming = true;
            ((Random) Reflect.get(Explosion.class, "ExplosionRNG", e)).setSeed(HarnessWorld.mix(hw.seed, -6));
            e.doExplosionA();
            e.doExplosionB(true);
        }
        l.put("destroyed", e.destroyedBlockPositions.size());
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
            case "weather": Dumps.weather(trace, t, w); break;
            default: throw new IllegalArgumentException("unknown dump " + d.get("what"));
        }
    }
}
