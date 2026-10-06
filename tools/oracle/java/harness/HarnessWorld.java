package harness;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;
import java.util.Random;
import java.util.stream.Stream;
import net.minecraft.src.Block;
import net.minecraft.src.BlockChest;
import net.minecraft.src.BlockDispenser;
import net.minecraft.src.BlockFurnace;
import net.minecraft.src.BlockRedstoneTorch;
import net.minecraft.src.BlockRedstoneWire;
import net.minecraft.src.BlockSand;
import net.minecraft.src.ChunkProvider;
import net.minecraft.src.Entity;
import net.minecraft.src.EntityChicken;
import net.minecraft.src.EntitySquid;
import net.minecraft.src.ISaveHandler;
import net.minecraft.src.Item;
import net.minecraft.src.ItemStack;
import net.minecraft.src.MathHelper;
import net.minecraft.src.NextTickListEntry;
import net.minecraft.src.RegionFileCache;
import net.minecraft.src.SaveHandlerMP;
import net.minecraft.src.SaveOldDir;
import net.minecraft.src.TileEntityPiston;
import net.minecraft.src.World;
import net.minecraft.src.WorldProvider;
import net.minecraft.src.WorldProviderHell;
import net.minecraft.src.WorldProviderSurface;

/**
 * A deterministic vanilla {@link World} for Tier B/C/S scenarios: the seeding protocol of
 * PLAN_v4 §4.2 / ADR-007 §4.3, documented in tools/oracle/README.md ("Seeding protocol") and
 * tools/oracle/census.md.
 */
final class HarnessWorld {
    /** Odd 64-bit constant of {@link #mix} (2^64 / golden ratio, as in SplitMix64). */
    static final long GOLDEN_GAMMA = 0x9E3779B97F4A7C15L;

    final World world;
    final long seed;
    final HarnessWorldAccess access;
    /** Temp directory holding the save directory {@code world/}, or null for SaveHandlerMP. */
    final Path saveRoot;
    final int difficulty;
    HarnessPlayer player;
    /** Minecraft.joinPlayerCounter (Minecraft.java:1135-1139). */
    int joinPlayerCounter;
    /**
     * Scenario field {@code "mob_spawning": "none"} (a Tier B isolation knob,
     * tests/golden/README.md): {@code setAllowedMobSpawns(false, false)} instead of the client's
     * {@code (difficulty > 0, true)}, so {@code SpawnerAnimals.performSpawning} returns at once
     * (SpawnerAnimals.java:20-21).
     */
    boolean noMobSpawning;
    /**
     * The left button's controller ({@code dig} op, TickScenario.dig), created by the first
     * {@code dig} op; null before.
     */
    Controller controller;
    /** Block {x, y, z} the button was held on last tick (a {@code dig} op), or null. */
    int[] lastDig;
    /** Minecraft.mouseTicksRan (Minecraft.java:162): the tick of the last left click. */
    int mouseTicksRan;
    /** Whether the current tick had a {@code dig} op. */
    boolean digThisTick;

    private HarnessWorld(World world, long seed, Path saveRoot, int difficulty) {
        this.world = world;
        this.seed = seed;
        this.saveRoot = saveRoot;
        this.difficulty = difficulty;
        this.access = new HarnessWorldAccess(seed);
        world.addWorldAccess(access);
    }

    /** {@code seed ^ (k * 0x9E3779B97F4A7C15)}, wrapping 64-bit arithmetic. */
    static long mix(long seed, long k) {
        return seed ^ (k * GOLDEN_GAMMA);
    }

    /** {@code entity.rand.setSeed(mix(seed, entity.entityId))} (Entity.rand, Entity.java:50, :96). */
    static void seedEntity(Entity e, long seed) {
        entityRand(e).setSeed(mix(seed, e.entityId));
    }

    /**
     * The join hook (HarnessWorldAccess.obtainEntitySkin): seed, re-draw the constructor's
     * non-persisted {@code rand} draws on the seeded stream, seed again. Re-drawn:
     * {@code EntityChicken.timeUntilNextEgg} (EntityChicken.java:17) and
     * {@code EntitySquid.rotationVelocity} (EntitySquid.java:23); neither is saved in NBT, so a
     * chunk reload (which constructs the entity anew) redraws them in vanilla too. Net effect,
     * as a Rust port states it: an entity's rand is seeded with mix(seed, id) at construction and
     * again whenever it joins the world.
     */
    static void seedOnJoin(Entity e, long seed) {
        seedEntity(e, seed);
        Random r = entityRand(e);
        if (e instanceof EntityChicken) {
            ((EntityChicken) e).timeUntilNextEgg = r.nextInt(6000) + 6000;
        } else if (e instanceof EntitySquid) {
            Reflect.set(EntitySquid.class, "rotationVelocity", e, 1.0F / (r.nextFloat() + 1.0F) * 0.2F);
        }
        seedEntity(e, seed);
    }

    static Random entityRand(Entity e) {
        return (Random) Reflect.get(Entity.class, "rand", e);
    }

    /**
     * Creates the world for a {@code tick} scenario. {@code saveFormat} "none" uses
     * {@code SaveHandlerMP} (no disk); "mcregion" a {@code SaveOldDir} on a fresh temp directory
     * (SaveOldDir.java:7-9, which writes {@code world/session.lock} and creates
     * {@code world/data}, {@code world/players}; SaveHandler.java:19-30).
     */
    static HarnessWorld create(long seed, String dimension, String worldType, String saveFormat, int difficulty) throws Exception {
        resetStatics(seed);
        WorldProvider provider = provider(dimension);
        Path root = null;
        ISaveHandler handler;
        switch (saveFormat) {
            case "none": handler = new SaveHandlerMP(); break;
            case "mcregion":
                root = Files.createTempDirectory("harness-save");
                handler = new SaveOldDir(root.toFile(), "world", true);
                break;
            default: throw new IllegalArgumentException("unknown save_format " + saveFormat);
        }
        // src: World.java:62-100. Fresh WorldInfo(seed, "harness"), spawn (0, 0, 0); no chunk
        // is loaded and no spawn point is searched for.
        World world = new World(handler, "harness", provider, seed);
        makeDeterministic(world, seed);
        switch (worldType) {
            case "generated": break;
            case "empty":
                // Swap the generator inside the vanilla ChunkProvider (ChunkProvider.java:14,
                // built by World.getChunkProvider, World.java:210-213). Safe right after
                // construction: the constructor loads no chunk (calculateInitialSkylight,
                // World.java:1732-1738, only reads the celestial angle) and the generator has
                // not been asked for anything yet.
                Reflect.set(ChunkProvider.class, "chunkProvider", world.getIChunkProvider(), new EmptyChunkProvider(world));
                break;
            default: throw new IllegalArgumentException("unknown world " + worldType);
        }
        return new HarnessWorld(world, seed, root, difficulty);
    }

    /**
     * Opens an existing save on a copy (world_read): {@code saveCopyRoot/world} must already be
     * the copy. Uses the loading constructor World(ISaveHandler, String, long, WorldProvider)
     * (World.java:149-208) with a null provider, so the dimension comes from level.dat.
     * {@code seed} only seeds the harness streams; the world seed is the file's.
     */
    static HarnessWorld open(Path saveCopyRoot, long seed) throws Exception {
        return open(saveCopyRoot, seed, 0);
    }

    /** {@link #open(Path, long)} with the tick scenario's {@code difficulty} ({@code "world": "load"}). */
    static HarnessWorld open(Path saveCopyRoot, long seed, int difficulty) throws Exception {
        resetStatics(seed);
        World world = new World(new SaveOldDir(saveCopyRoot.toFile(), "world", false), "harness", seed, null);
        if (world.isNewWorld) throw new IllegalStateException("no level.dat in " + saveCopyRoot.resolve("world"));
        makeDeterministic(world, seed);
        return new HarnessWorld(world, seed, saveCopyRoot, difficulty);
    }

    static WorldProvider provider(String dimension) {
        switch (dimension) {
            case "overworld": return new WorldProviderSurface();
            case "nether": return new WorldProviderHell();
            default: throw new IllegalArgumentException("unknown dimension " + dimension);
        }
    }

    /**
     * Removes the World constructor's unseeded randomness (PLAN_v4 §4.2, ADR-007 §4.3), all
     * derived from the scenario seed (identical to Tier A, tests/golden/README.md "terrain"):
     * {@code rand.setSeed(seed)} (World.java:82); {@code field_9437_g = new Random(seed).nextInt()}
     * (World.java:75, clock-seeded random-tick LCG state); {@code soundCounter = <same
     * Random>.nextInt(12000)} (World.java:90). {@code lockTimestamp} (World.java:80) only feeds
     * {@code World(World, WorldProvider)} (dimension change), never reached.
     */
    static void makeDeterministic(World world, long seed) {
        world.rand.setSeed(seed);
        Random derived = new Random(seed);
        Reflect.set(World.class, "field_9437_g", world, derived.nextInt());
        Reflect.set(World.class, "soundCounter", world, derived.nextInt(12000));
    }

    /**
     * Per-scenario reset of vanilla static state (census.md "Static and singleton state"):
     * {@code Entity.nextEntityID = 0} (Entity.java:7), {@code NextTickListEntry.nextTickEntryID = 0}
     * (NextTickListEntry.java:4), RegionFileCache closed and emptied (RegionFileCache.java:44-61),
     * and the unseeded singleton Randoms reseeded with {@code mix(seed, k)} for k = -1..-5:
     * {@code Item.itemRand} (Item.java:6), {@code Block.chest.random} (BlockChest.java:6),
     * {@code Block.stoneOvenIdle.furnaceRand}, {@code Block.stoneOvenActive.furnaceRand}
     * (BlockFurnace.java:6), {@code Block.dispenser.random} (BlockDispenser.java:6). Transient
     * static flags and lists are put back to their initial values: {@code BlockSand.fallInstantly}
     * (BlockSand.java:6), {@code BlockFurnace.keepFurnaceInventory} (BlockFurnace.java:8),
     * {@code BlockRedstoneTorch.torchUpdates} (BlockRedstoneTorch.java:9, burnout history keyed by
     * world time), {@code TileEntityPiston.field_31018_m} (TileEntityPiston.java:15),
     * {@code World.lightingUpdatesScheduled} (World.java:52). The instance state of the
     * {@code Block.redstoneWire} singleton is replaced: {@code wiresProvidePower = true}
     * (BlockRedstoneWire.java:9) and a fresh {@code blocksNeedingUpdate} HashSet
     * (BlockRedstoneWire.java:10), since HashSet.clear() (:48) keeps the grown table and its
     * iteration order (:47) depends on the capacity, so a set grown by an earlier scenario would
     * change the notification order of later ones.
     */
    static void resetStatics(long seed) throws Exception {
        Reflect.set(Entity.class, "nextEntityID", null, 0);
        Reflect.set(NextTickListEntry.class, "nextTickEntryID", null, 0L);
        RegionFileCache.func_22192_a();
        ((Random) Reflect.get(BlockChest.class, "random", Block.chest)).setSeed(mix(seed, -2));
        ((Random) Reflect.get(BlockFurnace.class, "furnaceRand", Block.stoneOvenIdle)).setSeed(mix(seed, -3));
        ((Random) Reflect.get(BlockFurnace.class, "furnaceRand", Block.stoneOvenActive)).setSeed(mix(seed, -4));
        ((Random) Reflect.get(BlockDispenser.class, "random", Block.dispenser)).setSeed(mix(seed, -5));
        // After Block: initialising Item first breaks vanilla's static-init order (Item ->
        // AchievementList -> ItemStack(Block.xxx) before Block exists).
        ((Random) Reflect.get(Item.class, "itemRand", null)).setSeed(mix(seed, -1));
        BlockSand.fallInstantly = false;
        Reflect.set(BlockFurnace.class, "keepFurnaceInventory", null, Boolean.FALSE);
        ((java.util.List<?>) Reflect.get(BlockRedstoneTorch.class, "torchUpdates", null)).clear();
        ((java.util.List<?>) Reflect.get(TileEntityPiston.class, "field_31018_m", null)).clear();
        Reflect.set(World.class, "lightingUpdatesScheduled", null, 0);
        Reflect.set(BlockRedstoneWire.class, "wiresProvidePower", Block.redstoneWire, Boolean.TRUE);
        Reflect.set(BlockRedstoneWire.class, "blocksNeedingUpdate", Block.redstoneWire, new java.util.HashSet());
    }

    /**
     * One client tick of the world, Minecraft.runTick's world part (Minecraft.java:1133-1174)
     * with {@code isGamePaused == false}, single player.
     */
    void runTick() {
        if (player != null) {
            // src: Minecraft.java:1134-1139
            if (++joinPlayerCounter == 30) {
                joinPlayerCounter = 0;
                world.joinEntityInSurroundings(player);
            }
        }
        world.difficultySetting = difficulty; // src: Minecraft.java:1142 (gameSettings.difficulty)
        if (world.field_27172_i > 0) --world.field_27172_i; // src: Minecraft.java:1155-1157
        world.updateEntities(); // src: Minecraft.java:1159; World.java:1178-1276
        if (noMobSpawning) {
            world.setAllowedMobSpawns(false, false); // "mob_spawning": "none"
        } else {
            world.setAllowedMobSpawns(difficulty > 0, true); // src: Minecraft.java:1163
        }
        world.tick(); // src: Minecraft.java:1164; World.java:1745-1780
        if (player != null) {
            // src: Minecraft.java:1167-1169. Draws 6000 world.rand values per tick
            // (World.java:1997-2011); the blocks' randomDisplayTick only reads the world and
            // emits particles/sounds from an unseeded local Random (World.java:1999).
            world.randomDisplayUpdates(MathHelper.floor_double(player.posX),
                    MathHelper.floor_double(player.posY), MathHelper.floor_double(player.posZ));
        }
    }

    /** Closes region files (RegionFileCache is static) and deletes the temp directory. */
    void close() throws IOException {
        RegionFileCache.func_22192_a();
        if (saveRoot != null) deleteTree(saveRoot);
    }

    static void deleteTree(Path root) throws IOException {
        if (!Files.exists(root)) return;
        try (Stream<Path> s = Files.walk(root)) {
            for (Path p : (Iterable<Path>) s.sorted(Comparator.reverseOrder())::iterator) Files.delete(p);
        }
    }

    static void copyTree(Path from, Path to) throws IOException {
        try (Stream<Path> s = Files.walk(from)) {
            for (Path p : (Iterable<Path>) s.sorted()::iterator) {
                Path q = to.resolve(from.relativize(p).toString());
                if (Files.isDirectory(p)) Files.createDirectories(q);
                else Files.copy(p, q);
            }
        }
    }

    /**
     * {@code PlayerControllerSP}'s block breaking (PlayerControllerSP.java:110-224,
     * PlayerController.java:21-32) for the harness player. The vanilla class reaches the world
     * and the player through {@code mc.theWorld} and {@code mc.thePlayer}, and
     * {@code Minecraft.thePlayer} is an {@code EntityPlayerSP}, which the harness cannot build
     * (HarnessPlayer); so this mirrors those methods line for line with the world and the player
     * as fields. Only {@code mc.sndManager.playSound} (the dig step sound, :177-179, no draws) is
     * left out.
     */
    static final class Controller {
        final World w;
        final HarnessPlayer p;
        int field_1074_c = -1;
        int field_1073_d = -1;
        int field_1072_e = -1;
        float curBlockDamage = 0.0F;
        float prevBlockDamage = 0.0F;
        float field_1069_h = 0.0F;
        int blockHitWait = 0;

        Controller(World w, HarnessPlayer p) {
            this.w = w;
            this.p = p;
        }

        /** PlayerController.sendBlockRemoved (PlayerController.java:21-32). */
        private boolean superSendBlockRemoved(int x, int y, int z, int side) {
            Block b = Block.blocksList[w.getBlockId(x, y, z)];
            w.playAuxSFX(2001, x, y, z, b.blockID + w.getBlockMetadata(x, y, z) * 256);
            int meta = w.getBlockMetadata(x, y, z);
            boolean removed = w.setBlockWithNotify(x, y, z, 0);
            if (b != null && removed) b.onBlockDestroyedByPlayer(w, x, y, z, meta);
            return removed;
        }

        /** PlayerControllerSP.sendBlockRemoved (PlayerControllerSP.java:126-145). */
        boolean sendBlockRemoved(int x, int y, int z, int side) {
            int id = w.getBlockId(x, y, z);
            int meta = w.getBlockMetadata(x, y, z);
            boolean removed = superSendBlockRemoved(x, y, z, side);
            ItemStack held = p.getCurrentEquippedItem();
            boolean canHarvest = p.canHarvestBlock(Block.blocksList[id]);
            if (held != null) {
                held.onDestroyBlock(id, x, y, z, p);
                if (held.stackSize == 0) {
                    held.onItemDestroyedByUse(p);
                    p.destroyCurrentEquippedItem();
                }
            }
            if (removed && canHarvest) Block.blocksList[id].harvestBlock(w, p, x, y, z, meta);
            return removed;
        }

        /**
         * PlayerControllerSP.clickBlock (PlayerControllerSP.java:147-158); returns what the
         * instant {@code sendBlockRemoved} returned, false when it did not run.
         */
        boolean clickBlock(int x, int y, int z, int side) {
            w.onBlockHit(p, x, y, z, side);
            int id = w.getBlockId(x, y, z);
            if (id > 0 && curBlockDamage == 0.0F) Block.blocksList[id].onBlockClicked(w, x, y, z, p);
            boolean broken = false;
            if (id > 0 && Block.blocksList[id].blockStrength(p) >= 1.0F) broken = sendBlockRemoved(x, y, z, side);
            return broken;
        }

        /** PlayerControllerSP.resetBlockRemoving (PlayerControllerSP.java:160-163). */
        void resetBlockRemoving() {
            curBlockDamage = 0.0F;
            blockHitWait = 0;
        }

        /**
         * PlayerControllerSP.sendBlockRemoving (PlayerControllerSP.java:165-199); returns what
         * {@code sendBlockRemoved} returned, false when it did not run.
         */
        boolean sendBlockRemoving(int x, int y, int z, int side) {
            boolean broken = false;
            if (blockHitWait > 0) {
                --blockHitWait;
            } else if (x == field_1074_c && y == field_1073_d && z == field_1072_e) {
                int id = w.getBlockId(x, y, z);
                if (id == 0) return false;
                Block b = Block.blocksList[id];
                curBlockDamage += b.blockStrength(p);
                // :177-179 mc.sndManager.playSound(step sound): client, no draws
                ++field_1069_h;
                if (curBlockDamage >= 1.0F) {
                    broken = sendBlockRemoved(x, y, z, side);
                    curBlockDamage = 0.0F;
                    prevBlockDamage = 0.0F;
                    field_1069_h = 0.0F;
                    blockHitWait = 5;
                }
            } else {
                curBlockDamage = 0.0F;
                prevBlockDamage = 0.0F;
                field_1069_h = 0.0F;
                field_1074_c = x;
                field_1073_d = y;
                field_1072_e = z;
            }
            return broken;
        }

        /** PlayerControllerSP.updateController (PlayerControllerSP.java:221-224), without the music. */
        void updateController() {
            prevBlockDamage = curBlockDamage;
        }
    }
}
