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

    /** Parses a {@code "0x…"} f32 bit-pattern string (8 hex digits); the bits are kept exactly. */
    static float parseF32(String s) {
        if (!s.startsWith("0x") || s.length() != 10) throw new IllegalArgumentException("not an f32 bit pattern: " + s);
        return Float.intBitsToFloat(Integer.parseUnsignedInt(s.substring(2), 16));
    }

    /** Parses a lower-case hex string (even length, no prefix) into bytes. */
    static byte[] parse(String s) {
        if (s.length() % 2 != 0) throw new IllegalArgumentException("odd-length hex string");
        byte[] b = new byte[s.length() / 2];
        for (int i = 0; i < b.length; i++) {
            int hi = Character.digit(s.charAt(2 * i), 16), lo = Character.digit(s.charAt(2 * i + 1), 16);
            if (hi < 0 || lo < 0 || Character.isUpperCase(s.charAt(2 * i)) || Character.isUpperCase(s.charAt(2 * i + 1))) {
                throw new IllegalArgumentException("not lower-case hex: " + s);
            }
            b[i] = (byte) (hi << 4 | lo);
        }
        return b;
    }

    private static String pad(String s, int width) {
        StringBuilder b = new StringBuilder(width);
        for (int i = s.length(); i < width; i++) b.append('0');
        return b.append(s).toString();
    }
}
