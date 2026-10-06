package harness;

import net.minecraft.src.Chunk;
import net.minecraft.src.IChunkProvider;
import net.minecraft.src.IProgressUpdate;
import net.minecraft.src.World;

/**
 * Terrain generator for {@code "world": "empty"} scenarios (tests/golden/README.md): every
 * chunk is all air. It replaces the world provider's generator (the {@code IChunkProvider} that
 * {@code WorldProvider.getChunkProvider} returns, e.g. ChunkProviderGenerate) inside the
 * vanilla {@code ChunkProvider} cache (ChunkProvider.java:14, field {@code chunkProvider}),
 * so chunk bookkeeping (chunkMap/chunkList, population flags, saving, unloading) stays vanilla.
 *
 * <p>{@link #provideChunk} mirrors ChunkProviderGenerate.provideChunk (ChunkProviderGenerate.java:193-204)
 * with no terrain: {@code new Chunk(world, new byte[32768], x, z)} (Chunk.java:48-54, zeroed
 * data/light arrays) then {@code generateSkylightMap()} (Chunk.java:88-131): height map all 0,
 * sky light 15 for y = 127..1 and 0 at y = 0 (the loop stops at {@code var7 > 0}), block light 0.
 * The remaining methods return what ChunkProviderGenerate returns (ChunkProviderGenerate.java:189-191,
 * :306-308, :605-619); {@code populate} is a no-op (the vanilla wrapper still marks the chunk
 * populated and modified, ChunkProvider.java:118-128).
 */
final class EmptyChunkProvider implements IChunkProvider {
    private final World world;

    EmptyChunkProvider(World world) {
        this.world = world;
    }

    @Override
    public boolean chunkExists(int x, int z) {
        return true;
    }

    @Override
    public Chunk provideChunk(int x, int z) {
        Chunk c = new Chunk(world, new byte[32768], x, z);
        c.generateSkylightMap();
        return c;
    }

    @Override
    public Chunk prepareChunk(int x, int z) {
        return provideChunk(x, z);
    }

    @Override
    public void populate(IChunkProvider provider, int x, int z) {
    }

    @Override
    public boolean saveChunks(boolean all, IProgressUpdate progress) {
        return true;
    }

    @Override
    public boolean unload100OldestChunks() {
        return false;
    }

    @Override
    public boolean canSave() {
        return true;
    }

    @Override
    public String makeString() {
        return "HarnessEmptyLevelSource";
    }
}
