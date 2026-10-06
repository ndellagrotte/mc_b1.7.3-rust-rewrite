package harness;

import java.util.Random;

/**
 * Replacement target for every vanilla {@code Math.random()} call
 * (tools/oracle/rewrite-math-random.sh). One seeded stream for the whole JVM;
 * the harness reseeds it at the start of every scenario.
 */
public final class HarnessMath {
    private static final Random STREAM = new Random(0L);

    private HarnessMath() {}

    public static void seed(long seed) {
        STREAM.setSeed(seed);
    }

    /** Same contract as {@code Math.random()}: uniform in [0, 1). */
    public static double random() {
        return STREAM.nextDouble();
    }
}
