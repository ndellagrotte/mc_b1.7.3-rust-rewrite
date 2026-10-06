package harness;

/** Lower-case hex and IEEE-754 bit-pattern strings (tests/golden/README.md, Encoding rules). */
final class Hex {
    private static final char[] DIGITS = "0123456789abcdef".toCharArray();

    private Hex() {}

    static String of(byte[] b) {
        char[] c = new char[b.length * 2];
        for (int i = 0; i < b.length; i++) {
            c[2 * i] = DIGITS[(b[i] >> 4) & 15];
            c[2 * i + 1] = DIGITS[b[i] & 15];
        }
        return new String(c);
    }

    /** {@code f32} as {@code "0x" + 8 hex digits} of the raw bits. */
    static String f32(float f) {
        return "0x" + pad(Integer.toHexString(Float.floatToRawIntBits(f)), 8);
    }

    /** {@code f64} as {@code "0x" + 16 hex digits} of the raw bits. */
    static String f64(double d) {
        return "0x" + pad(Long.toHexString(Double.doubleToRawLongBits(d)), 16);
    }

    /** Parses a {@code "0x…"} f64 bit-pattern string (16 hex digits). */
    static double parseF64(String s) {
        if (!s.startsWith("0x") || s.length() != 18) throw new IllegalArgumentException("not an f64 bit pattern: " + s);
        return Double.longBitsToDouble(Long.parseUnsignedLong(s.substring(2), 16));
    }

    private static String pad(String s, int width) {
        StringBuilder b = new StringBuilder(width);
        for (int i = s.length(); i < width; i++) b.append('0');
        return b.append(s).toString();
    }
}
