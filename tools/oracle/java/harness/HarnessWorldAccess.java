package harness;

import java.util.Map;
import java.util.TreeMap;
import net.minecraft.src.Entity;
import net.minecraft.src.EntityPlayer;
import net.minecraft.src.IWorldAccess;
import net.minecraft.src.TileEntity;

/**
 * The harness's only {@link IWorldAccess} (World.addWorldAccess, World.java:951). It replaces
 * the client's RenderGlobal: every callback is a no-op that only counts calls (for later
 * Tier D work; the counts are not traced, see tools/oracle/census.md), except
 * {@link #obtainEntitySkin}, which is the entity seeding hook of the seeding protocol.
 *
 * <p>Vanilla calls {@code obtainEntitySkin} from {@code World.entityJoinedWorld}
 * (World.java:896-918, the b1.7.3 name of "spawn entity in world", call at :915) and from
 * {@code World.addLoadedEntities} (World.java:2077-2083, entities of a chunk being loaded,
 * via Chunk.onChunkLoad, Chunk.java:459-467). Both fire after the entity has been added to
 * {@code loadedEntityList}. Weather effects ({@code World.addWeatherEffect}, World.java:891)
 * never pass through it.
 */
final class HarnessWorldAccess implements IWorldAccess {
    private final long seed;
    final Map<String, Long> calls = new TreeMap<>();
    long seeded;

    HarnessWorldAccess(long seed) {
        this.seed = seed;
    }

    private void count(String name) {
        calls.merge(name, 1L, Long::sum);
    }

    @Override
    public void obtainEntitySkin(Entity e) {
        count("obtainEntitySkin");
        HarnessWorld.seedOnJoin(e, seed);
        seeded++;
    }

    @Override
    public void markBlockAndNeighborsNeedsUpdate(int x, int y, int z) {
        count("markBlockAndNeighborsNeedsUpdate");
    }

    @Override
    public void markBlockRangeNeedsUpdate(int x0, int y0, int z0, int x1, int y1, int z1) {
        count("markBlockRangeNeedsUpdate");
    }

    @Override
    public void playSound(String name, double x, double y, double z, float volume, float pitch) {
        count("playSound");
    }

    @Override
    public void spawnParticle(String name, double x, double y, double z, double vx, double vy, double vz) {
        count("spawnParticle");
    }

    @Override
    public void releaseEntitySkin(Entity e) {
        count("releaseEntitySkin");
    }

    @Override
    public void updateAllRenderers() {
        count("updateAllRenderers");
    }

    @Override
    public void playRecord(String name, int x, int y, int z) {
        count("playRecord");
    }

    @Override
    public void doNothingWithTileEntity(int x, int y, int z, TileEntity te) {
        count("doNothingWithTileEntity");
    }

    @Override
    public void playAuxSFX(EntityPlayer player, int type, int x, int y, int z, int data) {
        count("playAuxSFX");
    }
}
