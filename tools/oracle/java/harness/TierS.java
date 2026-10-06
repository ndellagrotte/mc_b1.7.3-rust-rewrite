package harness;

import static harness.Main.Scenario.list;
import static harness.Main.Scenario.num;
import static harness.Main.Scenario.str;

import java.io.DataInputStream;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.RandomAccessFile;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Stream;
import net.minecraft.src.Chunk;
import net.minecraft.src.CompressedStreamTools;
import net.minecraft.src.IInventory;
import net.minecraft.src.ItemStack;
import net.minecraft.src.McRegionChunkLoader;
import net.minecraft.src.NBTBase;
import net.minecraft.src.NBTTagCompound;
import net.minecraft.src.NBTTagList;
import net.minecraft.src.RegionFileCache;
import net.minecraft.src.TileEntity;
import net.minecraft.src.WorldInfo;

/**
 * Tier S: the {@code save} op of {@code tick} scenarios and the {@code world_read} kind
 * (tests/golden/README.md "Tier S"). Compared as parsed trees, never as raw file bytes
 * (PLAN_v4 §4.3: compound order is HashMap order, gzip/zlib output is implementation specific).
 */
final class TierS {
    private TierS() {}

    /** level.dat fields whose value is not reproducible: wall clock, and the sum of deflate-dependent sector counts. */
    private static final Set<String> MASKED = Set.of("LastPlayed", "SizeOnDisk");

    // ---- save op ----------------------------------------------------------------

    /**
     * {@code World.saveWorld(true, null)} (World.java:277-290): saveLevel (session lock check,
     * {@code SaveOldDir.saveWorldInfoAndPlayer}, which sets version 19132, SaveOldDir.java:22-25,
     * SaveHandler.java:109-136), then {@code ChunkProvider.saveChunks(true, null)}
     * (ChunkProvider.java:130-158: every chunk with {@code needsSaving(true)}, Chunk.java:536-550,
     * through McRegionChunkLoader.saveChunk, McRegionChunkLoader.java:42-58). Then the files are
     * read back with the vanilla readers.
     */
    static void save(HarnessWorld hw, Map<String, Object> op, long t, Trace trace) throws Exception {
        hw.world.saveWorld(true, null);
        Path dir = hw.saveRoot.resolve("world");
        emitFiles(t, dir, op, trace, true, hw);
    }

    // ---- world_read kind --------------------------------------------------------

    /**
     * Reads a committed world folder (scenario sidecar directory {@code dir}) through a copy in a
     * temp directory: opening a save writes session.lock (SaveHandler.java:19-45).
     */
    static Trace worldRead(Map<String, Object> sc, Path scenarioDir) throws Exception {
        String sub = str(sc, "dir");
        if (sub.contains("/") || sub.contains("\\") || sub.startsWith(".")) throw new IllegalArgumentException("\"dir\" must be a plain name");
        Path src = scenarioDir.resolve(sub);
        if (!Files.isRegularFile(src.resolve("level.dat"))) throw new IllegalArgumentException("no " + sub + "/level.dat");
        // No scenario seed: the harness streams use 0; the world seed is level.dat's.
        HarnessMath.seed(0L);
        Path root = Files.createTempDirectory("harness-read");
        HarnessWorld hw = null;
        Trace trace = new Trace();
        try {
            HarnessWorld.copyTree(src, root.resolve("world"));
            hw = HarnessWorld.open(root, 0L);
            emitFiles(0, root.resolve("world"), sc, trace, false, hw);
        } finally {
            if (hw != null) hw.close();
            else HarnessWorld.deleteTree(root);
        }
        return trace;
    }

    // ---- shared -----------------------------------------------------------------

    /**
     * level_dat, world_info, one region_header per region file (name order), then per chunk:
     * chunk_nbt, chunk_digest (world_read only), tile_entity lines.
     */
    private static void emitFiles(long t, Path dir, Map<String, Object> spec, Trace trace, boolean mask, HarnessWorld hw)
            throws Exception {
        NBTTagCompound root;
        try (InputStream in = new FileInputStream(dir.resolve("level.dat").toFile())) {
            root = CompressedStreamTools.loadGzippedCompoundFromOutputStream(in); // SaveHandler.java:85
        }
        NBTTagCompound data = root.getCompoundTag("Data");
        List<String> masked = new ArrayList<>();
        NBTTagCompound shown = root;
        if (mask) {
            NBTTagCompound copy = new NBTTagCompound();
            for (Object o : data.getTags()) {
                NBTBase tag = (NBTBase) o;
                if (MASKED.contains(tag.getKey())) masked.add("Data." + tag.getKey());
                else copy.setTag(tag.getKey(), tag);
            }
            shown = new NBTTagCompound();
            shown.setKey(root.getKey());
            shown.setTag("Data", copy);
            masked.sort(null);
        }
        StringBuilder m = new StringBuilder("[");
        for (String s : masked) {
            if (m.length() > 1) m.append(',');
            Json.quote(m, s);
        }
        trace.add(Dumps.line(t, "level_dat").raw("root", Nbt.canonicalRoot(shown)).raw("masked", m.append(']').toString()));

        WorldInfo wi = new WorldInfo(data); // WorldInfo.java:22-40
        trace.add(Dumps.line(t, "world_info").put("RandomSeed", wi.getRandomSeed()).put("SpawnX", wi.getSpawnX())
                .put("SpawnY", wi.getSpawnY()).put("SpawnZ", wi.getSpawnZ()).put("Time", wi.getWorldTime())
                .put("LevelName", wi.getWorldName()).put("version", wi.getSaveVersion())
                .put("rainTime", wi.getRainTime()).put("raining", wi.getRaining())
                .put("thunderTime", wi.getThunderTime()).put("thundering", wi.getThundering())
                .put("dimension", wi.getDimension()).put("player", wi.getPlayerNBTTagCompound() != null));

        Path regionDir = dir.resolve("region");
        List<Path> regions = new ArrayList<>();
        if (Files.isDirectory(regionDir)) {
            try (Stream<Path> s = Files.list(regionDir)) {
                s.filter(p -> p.getFileName().toString().endsWith(".mcr")).sorted().forEach(regions::add);
            }
        }
        List<int[]> present = new ArrayList<>();
        for (Path r : regions) present.addAll(regionHeader(t, r, trace));

        List<int[]> chunks = new ArrayList<>();
        if (spec.containsKey("chunks")) {
            for (Object o : list(spec, "chunks")) {
                Map<String, Object> c = Dumps.map(o, "chunk");
                chunks.add(new int[] {Math.toIntExact(num(c, "x")), Math.toIntExact(num(c, "z"))});
            }
        } else {
            chunks = present;
        }
        McRegionChunkLoader loader = new McRegionChunkLoader(dir.toFile());
        for (int[] c : chunks) {
            // Raw stored tree, read as McRegionChunkLoader.loadChunk does (McRegionChunkLoader.java:15-17).
            DataInputStream in = RegionFileCache.getChunkInputStream(dir.toFile(), c[0], c[1]);
            if (in == null) {
                trace.add(Dumps.line(t, "chunk_absent").put("x", c[0]).put("z", c[1]));
                continue;
            }
            NBTTagCompound chunk;
            try {
                chunk = CompressedStreamTools.read(in);
            } finally {
                in.close();
            }
            trace.add(Dumps.line(t, "chunk_nbt").put("x", c[0]).put("z", c[1]).raw("root", Nbt.canonicalRoot(chunk)));
            if (!mask) {
                // world_read only: the Chunk the vanilla loader builds (ChunkLoader.loadChunkIntoWorldFromCompound,
                // ChunkLoader.java:151-210, then removeUnknownBlocks). Not done after a save op: it constructs
                // the chunk's entities (Entity ids, Math.random draws) and would disturb a running scenario.
                Chunk loaded = loader.loadChunk(hw.world, c[0], c[1]);
                TierA.chunkDigest(trace, t, loaded, c[0], c[1], "loaded", false);
            }
            NBTTagList tes = chunk.getCompoundTag("Level").getTagList("TileEntities");
            for (int i = 0; i < tes.tagCount(); i++) {
                tileEntity(t, (NBTTagCompound) tes.tagAt(i), i, trace);
            }
        }
    }

    /**
     * The region file's header (RegionFile.java:31-80): 1024 big-endian location ints
     * ({@code sector << 8 | sectorCount}, index {@code x + z * 32}) then 1024 timestamps
     * (wall-clock seconds, RegionFile.java:232). Traced: the indices with a non-zero location,
     * and whether exactly those have a non-zero timestamp. Sector numbers and counts depend on the
     * deflate output (RegionFile.java:160-230), so the full table and the file length go into
     * {@code dbg} (never compared).
     */
    private static List<int[]> regionHeader(long t, Path file, Trace trace) throws IOException {
        int[] loc = new int[1024], ts = new int[1024];
        long len;
        try (RandomAccessFile f = new RandomAccessFile(file.toFile(), "r")) {
            len = f.length();
            for (int i = 0; i < 1024; i++) loc[i] = f.readInt();
            for (int i = 0; i < 1024; i++) ts[i] = f.readInt();
        }
        String name = file.getFileName().toString();
        String[] parts = name.split("\\.");
        int rx = Integer.parseInt(parts[1]), rz = Integer.parseInt(parts[2]);
        StringBuilder idx = new StringBuilder("["), table = new StringBuilder("[");
        boolean consistent = true;
        List<int[]> present = new ArrayList<>();
        for (int i = 0; i < 1024; i++) {
            if (i > 0) table.append(',');
            table.append(loc[i]);
            if (loc[i] != 0) {
                if (idx.length() > 1) idx.append(',');
                idx.append(i);
                present.add(new int[] {rx * 32 + (i & 31), rz * 32 + (i >> 5)});
            }
            if ((loc[i] != 0) != (ts[i] != 0)) consistent = false;
        }
        Json.Line dbg = new Json.Line().put("len", len).raw("locations", table.append(']').toString());
        trace.add(Dumps.line(t, "region_header").put("name", name).put("count", present.size())
                .raw("present", idx.append(']').toString()).put("timestamps_match", consistent)
                .raw("dbg", dbg.toString()));
        return present;
    }

    /**
     * One stored tile entity, through the vanilla reader {@code TileEntity.createAndLoadEntity}
     * (TileEntity.java:45-63), in the chunk's TileEntities list order (written in
     * chunkTileEntityMap HashMap order, ChunkLoader.java:136-148). For an IInventory, the
     * non-empty slots in slot order.
     */
    private static void tileEntity(long t, NBTTagCompound nbt, int index, Trace trace) {
        TileEntity te = TileEntity.createAndLoadEntity(nbt);
        Json.Line l = Dumps.line(t, "tile_entity").put("index", index).put("id", nbt.getString("id"));
        if (te == null) {
            trace.add(l.put("known", false));
            return;
        }
        l.put("known", true).put("class", te.getClass().getSimpleName())
                .put("x", te.xCoord).put("y", te.yCoord).put("z", te.zCoord);
        if (te instanceof IInventory) {
            IInventory inv = (IInventory) te;
            StringBuilder items = new StringBuilder("[");
            for (int s = 0; s < inv.getSizeInventory(); s++) {
                ItemStack st = inv.getStackInSlot(s);
                if (st == null) continue;
                if (items.length() > 1) items.append(',');
                items.append(new Json.Line().put("slot", s).put("id", st.itemID).put("count", st.stackSize)
                        .put("damage", st.getItemDamage()));
            }
            l.put("size", inv.getSizeInventory()).raw("items", items.append(']').toString());
        }
        trace.add(l);
    }
}
