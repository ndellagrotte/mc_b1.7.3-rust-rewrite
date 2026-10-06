package harness;

import java.nio.file.Path;
import java.util.Map;
import net.minecraft.src.Block;
import net.minecraft.src.EntityFallingSand;
import net.minecraft.src.EntityItem;
import net.minecraft.src.Item;
import net.minecraft.src.ItemStack;
import net.minecraft.src.TileEntityChest;
import net.minecraft.src.TileEntityFurnace;
import net.minecraft.src.TileEntitySign;
import net.minecraft.src.World;

/**
 * Tier S cross-read checks (WP-1.9b; not a fixture kind). Run by
 * {@code crates/mc-sim/tests/golden_save.rs} when the oracle is built.
 *
 * <pre>
 * java -cp "build/oracle:$CP" harness.WorldReadDir read  &lt;parent&gt;
 * java -cp "build/oracle:$CP" harness.WorldReadDir write &lt;parent&gt;
 * </pre>
 *
 * <p>{@code read}: the save {@code <parent>/world} (written by the Rust port) read by the vanilla
 * loaders exactly as a {@code world_read} scenario reads its sidecar ({@link TierS#worldRead}:
 * a temp copy opened with {@code World(SaveOldDir, "harness", 0, null)}; every present chunk), the
 * trace lines printed to stdout.
 *
 * <p>{@code write}: a vanilla world saved into {@code <parent>/world} for the Rust port to read:
 * {@link HarnessWorld#create} (seed 123, overworld, empty generator, {@code SaveOldDir}), chunks
 * -2..2 preloaded, a stone platform in chunk (0, 0), a chest (2 planks in slot 0, 4 sticks in slot
 * 5), a furnace (3 coal in the fuel slot), a sign post with text, an {@code EntityItem} (2
 * diamonds) and an {@code EntityFallingSand} (sand) joined, then {@code World.saveWorld(true,
 * null)} (World.java:278-291) and the save directory copied out.
 */
public final class WorldReadDir {
    private WorldReadDir() {}

    public static void main(String[] args) throws Exception {
        if (args.length != 2 || !(args[0].equals("read") || args[0].equals("write"))) {
            System.err.println("usage: harness.WorldReadDir read|write <parent>");
            System.exit(2);
        }
        Path parent = Path.of(args[1]);
        if (args[0].equals("read")) {
            Trace t = TierS.worldRead(Map.of("dir", "world"), parent);
            StringBuilder b = new StringBuilder();
            for (String l : t.lines) b.append(l).append('\n');
            System.out.print(b);
        } else {
            write(parent);
        }
    }

    private static void write(Path parent) throws Exception {
        HarnessWorld hw = HarnessWorld.create(123L, "overworld", "empty", "mcregion", 0);
        try {
            World w = hw.world;
            for (int dz = -2; dz <= 2; dz++) {
                for (int dx = -2; dx <= 2; dx++) {
                    w.getChunkFromChunkCoords(dx, dz);
                    while (w.updatingLighting()) {
                    }
                }
            }
            for (int x = 0; x < 16; x++) {
                for (int z = 0; z < 16; z++) {
                    w.setBlockAndMetadata(x, 60, z, Block.stone.blockID, 0);
                }
            }
            w.setBlockAndMetadataWithNotify(3, 61, 3, Block.chest.blockID, 0);
            TileEntityChest chest = (TileEntityChest) w.getBlockTileEntity(3, 61, 3);
            chest.setInventorySlotContents(0, new ItemStack(Block.planks, 2));
            chest.setInventorySlotContents(5, new ItemStack(Item.stick, 4));
            w.setBlockAndMetadataWithNotify(5, 61, 3, Block.stoneOvenIdle.blockID, 0);
            TileEntityFurnace furnace = (TileEntityFurnace) w.getBlockTileEntity(5, 61, 3);
            furnace.setInventorySlotContents(1, new ItemStack(Item.coal, 3));
            w.setBlockAndMetadataWithNotify(7, 61, 3, Block.signPost.blockID, 4);
            TileEntitySign sign = (TileEntitySign) w.getBlockTileEntity(7, 61, 3);
            sign.signText[0] = "cross";
            sign.signText[3] = "read";
            w.entityJoinedWorld(new EntityItem(w, 8.5D, 61.0D, 8.5D, new ItemStack(Item.diamond, 2)));
            w.entityJoinedWorld(new EntityFallingSand(w, 10.5D, 70.5D, 10.5D, Block.sand.blockID));
            w.saveWorld(true, null);
            HarnessWorld.copyTree(hw.saveRoot.resolve("world"), parent.resolve("world"));
        } finally {
            hw.close();
        }
    }
}
