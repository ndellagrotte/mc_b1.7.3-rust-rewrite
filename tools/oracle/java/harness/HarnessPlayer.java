package harness;

import net.minecraft.src.EntityPlayer;
import net.minecraft.src.EnumStatus;
import net.minecraft.src.MovementInput;
import net.minecraft.src.World;

/**
 * A client-free player (PLAN_v4 §4.2). {@code EntityPlayerSP} needs a {@code Minecraft}
 * (EntityPlayerSP.java:12-21, and its onLivingUpdate reads mc.statFileWriter, mc.currentScreen
 * and movementInput, :34-87), so the harness subclasses {@code EntityPlayer} directly. In
 * b1.7.3 its only abstract method is {@code func_6420_o} (EntityPlayer.java:519), a no-op in
 * EntityPlayerSP (:169-170).
 *
 * <p>The {@code EntityPlayerSP} overrides that need no client are reproduced here: after
 * {@code EntityPlayer.updatePlayerActionState} (swing progress, EntityPlayer.java:158-170) the
 * strafe/forward/jump inputs come from {@link #movementInput} (EntityPlayerSP.java:27-32), and
 * {@code isSneaking} is {@code movementInput.sneak && !sleeping} (EntityPlayerSP.java:144-146).
 * {@code movementInput} is all zero until an {@code input} op sets it (TickScenario). The
 * username is the one a {@code Session} would give an {@code EntityPlayerSP}
 * (EntityPlayerSP.java:20): "Player", so {@code EntityPlayer.onDeath}'s
 * {@code username.equals("Notch")} (EntityPlayer.java:223) does not throw.
 * {@link #sleepInBedAt} keeps its result for the {@code sleep} op (BlockBed.blockActivated
 * discards it, BlockBed.java:70-80).
 *
 * <p>Not reproduced (client only, documented in tools/oracle/README.md): the rest of
 * {@code EntityPlayerSP.onLivingUpdate} (EntityPlayerSP.java:34-86: portal timing and travel,
 * the sneak {@code ySize} dip, {@code pushOutOfBlocks}), statistics and achievements
 * ({@code addStat} is EntityPlayer's no-op, EntityPlayer.java:718), pickup effects.
 *
 * <p>Vanilla paths that need a player in {@code World.playerEntities}: random block ticks,
 * snow/ice and lightning (World.updateBlocksAndPlayCaveSounds, World.java:1868-1965, chunks
 * within 9 of a player), mob spawning (SpawnerAnimals.performSpawning, SpawnerAnimals.java:19-38,
 * chunks within 8), sleeping (World.java:1747-1760), cave sounds, {@code getClosestPlayer}
 * users (mob AI, despawning, EntityLiving.despawnEntity), item pickup, and the client-side
 * {@code World.randomDisplayUpdates} (World.java:1997-2011) the tick loop runs for it.
 */
public final class HarnessPlayer extends EntityPlayer {
    /** EntityPlayerSP.movementInput (EntityPlayerSP.java:6), set by the {@code input} op. */
    final MovementInput movementInput = new MovementInput();
    /** The last {@link #sleepInBedAt} result, or null. */
    EnumStatus lastSleepStatus;

    public HarnessPlayer(World world) {
        super(world);
        this.username = "Player";
    }

    @Override
    public void func_6420_o() {
    }

    @Override
    protected void updatePlayerActionState() {
        super.updatePlayerActionState();
        // src: EntityPlayerSP.java:29-31
        this.moveStrafing = movementInput.moveStrafe;
        this.moveForward = movementInput.moveForward;
        this.isJumping = movementInput.jump;
    }

    @Override
    public boolean isSneaking() {
        return movementInput.sneak && !this.sleeping; // src: EntityPlayerSP.java:144-146
    }

    @Override
    public EnumStatus sleepInBedAt(int x, int y, int z) {
        EnumStatus s = super.sleepInBedAt(x, y, z);
        lastSleepStatus = s;
        return s;
    }
}
