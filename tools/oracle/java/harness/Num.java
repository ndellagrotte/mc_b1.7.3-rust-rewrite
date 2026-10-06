package harness;

import java.util.List;

/**
 * Scenario numbers that are floats in Java: a JSON integer (exact) or a bit-pattern string
 * ({@code "0x" + 16} hex digits for f64, {@code "0x" + 8} for f32; tests/golden/README.md,
 * Encoding rules). The scenario reader rejects decimal floats.
 */
final class Num {
    private Num() {}

    static double f64(Object v) {
        if (v instanceof Long) return (double) (Long) v;
        if (v instanceof String) return Hex.parseF64((String) v);
        throw new IllegalArgumentException("expected an integer or an f64 bit pattern, got " + v);
    }

    static float f32(Object v) {
        if (v instanceof Long) return (float) (long) (Long) v;
        if (v instanceof String) return Hex.parseF32((String) v);
        throw new IllegalArgumentException("expected an integer or an f32 bit pattern, got " + v);
    }

    static double[] vec3(Object v) {
        if (!(v instanceof List) || ((List<?>) v).size() != 3) throw new IllegalArgumentException("expected [x, y, z], got " + v);
        List<?> l = (List<?>) v;
        return new double[] {f64(l.get(0)), f64(l.get(1)), f64(l.get(2))};
    }
}
