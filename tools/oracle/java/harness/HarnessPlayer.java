package harness;

import net.minecraft.src.EntityPlayer;
import net.minecraft.src.World;

/**
 * A client-free player (PLAN_v4 §4.2). {@code EntityPlayerSP} needs a {@code Minecraft}
 * (EntityPlayerSP.java:12-21, and its onLivingUpdate reads mc.statFileWriter, mc.currentScreen
 * and movementInput, :34-87), so the harness subclasses {@code EntityPlayer} directly. In
 * b1.7.3 its only abstract method is {@code func_6420_o} (EntityPlayer.java:519), a no-op in
 * EntityPlayerSP (:169-170).
 *
 * <p>Behaviour is EntityPlayerSP with an idle {@code MovementInput} (no keys): after
 * {@code EntityPlayer.updatePlayerActionState} (swing progress, EntityPlayer.java:158-170)
 * the strafe/forward/jump inputs are zero (EntityPlayerSP.java:27-32). Not reproduced (client
 * only, documented in tools/oracle/README.md): portal timing and travel, {@code pushOutOfBlocks}
 * (EntityPlayerSP.java:80-83), sneaking, statistics and achievements ({@code addStat} is
 * EntityPlayer's no-op, EntityPlayer.java:718), pickup effects.
 *
 * <p>Vanilla paths that need a player in {@code World.playerEntities}: random block ticks,
 * snow/ice and lightning (World.updateBlocksAndPlayCaveSounds, World.java:1868-1965, chunks
 * within 9 of a player), mob spawning (SpawnerAnimals.performSpawning, SpawnerAnimals.java:19-38,
 * chunks within 8), sleeping (World.java:1747-1760), cave sounds, {@code getClosestPlayer}
 * users (mob AI, despawning, EntityLiving.despawnEntity), item pickup, and the client-side
 * {@code World.randomDisplayUpdates} (World.java:1997-2011) the tick loop runs for it.
 */
public final class HarnessPlayer extends EntityPlayer {
    public HarnessPlayer(World world) {
        super(world);
    }

    @Override
    public void func_6420_o() {
    }

    @Override
    protected void updatePlayerActionState() {
        super.updatePlayerActionState();
        this.moveStrafing = 0.0F;
        this.moveForward = 0.0F;
        this.isJumping = false;
    }
}
