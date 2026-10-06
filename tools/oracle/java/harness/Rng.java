package harness;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.ObjectOutputStream;
import java.util.Random;

/**
 * Reads the 48-bit internal state of a {@code java.util.Random} without
 * {@code --add-opens}: the JDK serial form of {@code Random} (its {@code writeObject} puts the
 * fields {@code haveNextNextGaussian} (boolean), {@code nextNextGaussian} (double),
 * {@code seed} (long), written in that order, the descriptor's sorted primitive order) ends
 * with the 8 big-endian bytes of {@code seed} followed by {@code TC_ENDBLOCKDATA} (0x78).
 * {@link #selfTest} checks this against {@code new Random(42)} at start-up.
 */
final class Rng {
    private static final long MULTIPLIER = 0x5DEECE66DL;
    private static final long MASK = (1L << 48) - 1;

    private Rng() {}

    /** The {@code seed} field (48-bit LCG state) of an exact {@code java.util.Random}. */
    static long state(Random r) {
        if (r.getClass() != Random.class) {
            throw new IllegalArgumentException("not a plain java.util.Random: " + r.getClass());
        }
        byte[] a;
        try (ByteArrayOutputStream b = new ByteArrayOutputStream(); ObjectOutputStream o = new ObjectOutputStream(b)) {
            o.writeObject(r);
            o.flush();
            a = b.toByteArray();
        } catch (IOException e) {
            throw new IllegalStateException(e);
        }
        int n = a.length;
        if (n < 18 || a[n - 1] != 0x78) throw new IllegalStateException("unexpected Random serial form");
        long s = 0;
        for (int i = n - 9; i < n - 1; i++) s = (s << 8) | (a[i] & 0xff);
        return s;
    }

    /** Fails unless {@link #state} reads the documented LCG state (java.util.Random contract). */
    static void selfTest() {
        Random r = new Random(42L);
        long expect = (42L ^ MULTIPLIER) & MASK;
        if (state(r) != expect) throw new IllegalStateException("Rng.state self-test failed (initial)");
        r.nextInt();
        expect = (expect * MULTIPLIER + 0xBL) & MASK;
        if (state(r) != expect) throw new IllegalStateException("Rng.state self-test failed (after nextInt)");
    }
}
