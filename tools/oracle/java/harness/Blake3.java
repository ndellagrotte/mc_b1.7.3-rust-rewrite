package harness;

/**
 * BLAKE3 (https://github.com/BLAKE3-team/BLAKE3-specs), hash mode only, 32-byte
 * output. Whole-input, recursive tree form: the left subtree always holds the
 * largest power-of-two number of 1024-byte chunks that leaves at least one byte
 * for the right subtree, which is exactly the tree the spec defines.
 * Validated by {@link #selfTest()} against the official test vectors.
 */
final class Blake3 {
    private static final int[] IV = {
        0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
        0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
    };
    private static final int[] PERM = {2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8};
    private static final int CHUNK_START = 1, CHUNK_END = 2, PARENT = 4, ROOT = 8;
    private static final int CHUNK_LEN = 1024, BLOCK_LEN = 64;

    private Blake3() {}

    static byte[] hash(byte[] in) {
        int[] cv = node(in, 0, in.length, 0L, true);
        byte[] out = new byte[32];
        for (int i = 0; i < 8; i++) {
            for (int b = 0; b < 4; b++) out[i * 4 + b] = (byte) (cv[i] >>> (8 * b));
        }
        return out;
    }

    static String hex(byte[] in) {
        return Hex.of(hash(in));
    }

    private static int[] node(byte[] in, int off, int len, long chunk, boolean root) {
        if (len <= CHUNK_LEN) return chunk(in, off, len, chunk, root);
        long chunks = (len + CHUNK_LEN - 1) / CHUNK_LEN;
        long leftChunks = Long.highestOneBit(chunks - 1);
        int leftLen = (int) (leftChunks * CHUNK_LEN);
        int[] l = node(in, off, leftLen, chunk, false);
        int[] r = node(in, off + leftLen, len - leftLen, chunk + leftChunks, false);
        int[] m = new int[16];
        System.arraycopy(l, 0, m, 0, 8);
        System.arraycopy(r, 0, m, 8, 8);
        return compress(IV, m, 0L, BLOCK_LEN, PARENT | (root ? ROOT : 0));
    }

    private static int[] chunk(byte[] in, int off, int len, long counter, boolean root) {
        int blocks = Math.max(1, (len + BLOCK_LEN - 1) / BLOCK_LEN);
        int[] cv = IV;
        for (int b = 0; b < blocks; b++) {
            int start = b * BLOCK_LEN;
            int blockLen = Math.min(BLOCK_LEN, len - start);
            int[] m = new int[16];
            for (int i = 0; i < blockLen; i++) {
                m[i >> 2] |= (in[off + start + i] & 0xFF) << (8 * (i & 3));
            }
            boolean last = b == blocks - 1;
            int flags = (b == 0 ? CHUNK_START : 0) | (last ? CHUNK_END : 0) | (last && root ? ROOT : 0);
            cv = compress(cv, m, counter, blockLen, flags);
        }
        return cv;
    }

    /** Returns the first 8 output words (the chaining value / root hash words). */
    private static int[] compress(int[] cv, int[] m, long counter, int blockLen, int flags) {
        int[] s = {
            cv[0], cv[1], cv[2], cv[3], cv[4], cv[5], cv[6], cv[7],
            IV[0], IV[1], IV[2], IV[3], (int) counter, (int) (counter >>> 32), blockLen, flags,
        };
        int[] w = m.clone();
        for (int round = 0; round < 7; round++) {
            g(s, 0, 4, 8, 12, w[0], w[1]);
            g(s, 1, 5, 9, 13, w[2], w[3]);
            g(s, 2, 6, 10, 14, w[4], w[5]);
            g(s, 3, 7, 11, 15, w[6], w[7]);
            g(s, 0, 5, 10, 15, w[8], w[9]);
            g(s, 1, 6, 11, 12, w[10], w[11]);
            g(s, 2, 7, 8, 13, w[12], w[13]);
            g(s, 3, 4, 9, 14, w[14], w[15]);
            int[] p = new int[16];
            for (int i = 0; i < 16; i++) p[i] = w[PERM[i]];
            w = p;
        }
        int[] out = new int[8];
        for (int i = 0; i < 8; i++) out[i] = s[i] ^ s[i + 8];
        return out;
    }

    private static void g(int[] s, int a, int b, int c, int d, int x, int y) {
        s[a] = s[a] + s[b] + x;
        s[d] = Integer.rotateRight(s[d] ^ s[a], 16);
        s[c] = s[c] + s[d];
        s[b] = Integer.rotateRight(s[b] ^ s[c], 12);
        s[a] = s[a] + s[b] + y;
        s[d] = Integer.rotateRight(s[d] ^ s[a], 8);
        s[c] = s[c] + s[d];
        s[b] = Integer.rotateRight(s[b] ^ s[c], 7);
    }

    /**
     * Official test vectors (BLAKE3 repo, test_vectors/test_vectors.json, every input
     * length up to 102400): input byte i is {@code i % 251}; expected = the first 32
     * bytes of the hash. Cross-checked against the reference C implementation.
     */
    private static final Object[][] VECTORS = {
        {0, "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"},
        {1, "2d3adedff11b61f14c886e35afa036736dcd87a74d27b5c1510225d0f592e213"},
        {1023, "10108970eeda3eb932baac1428c7a2163b0e924c9a9e25b35bba72b28f70bd11"},
        {1024, "42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7"},
        {1025, "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444"},
        {2048, "e776b6028c7cd22a4d0ba182a8bf62205d2ef576467e838ed6f2529b85fba24a"},
        {2049, "5f4d72f40d7a5f82b15ca2b2e44b1de3c2ef86c426c95c1af0b6879522563030"},
        {3072, "b98cb0ff3623be03326b373de6b9095218513e64f1ee2edd2525c7ad1e5cffd2"},
        {3073, "7124b49501012f81cc7f11ca069ec9226cecb8a2c850cfe644e327d22d3e1cd3"},
        {4096, "015094013f57a5277b59d8475c0501042c0b642e531b0a1c8f58d2163229e969"},
        {4097, "9b4052b38f1c5fc8b1f9ff7ac7b27cd242487b3d890d15c96a1c25b8aa0fb995"},
        {5120, "9cadc15fed8b5d854562b26a9536d9707cadeda9b143978f319ab34230535833"},
        {5121, "628bd2cb2004694adaab7bbd778a25df25c47b9d4155a55f8fbd79f2fe154cff"},
        {6144, "3e2e5b74e048f3add6d21faab3f83aa44d3b2278afb83b80b3c35164ebeca205"},
        {6145, "f1323a8631446cc50536a9f705ee5cb619424d46887f3c376c695b70e0f0507f"},
        {7168, "61da957ec2499a95d6b8023e2b0e604ec7f6b50e80a9678b89d2628e99ada77a"},
        {7169, "a003fc7a51754a9b3c7fae0367ab3d782dccf28855a03d435f8cfe74605e7817"},
        {8192, "aae792484c8efe4f19e2ca7d371d8c467ffb10748d8a5a1ae579948f718a2a63"},
        {8193, "bab6c09cb8ce8cf459261398d2e7aef35700bf488116ceb94a36d0f5f1b7bc3b"},
        {16384, "f875d6646de28985646f34ee13be9a576fd515f76b5b0a26bb324735041ddde4"},
        {31744, "62b6960e1a44bcc1eb1a611a8d6235b6b4b78f32e7abc4fb4c6cdcce94895c47"},
        {102400, "bc3e3d41a1146b069abffad3c0d44860cf664390afce4d9661f7902e7943e085"},
    };

    /** Returns the number of failing vectors (0 = pass); {@code verbose} prints every vector. */
    static int selfTest(boolean verbose) {
        int failures = 0;
        for (Object[] v : VECTORS) {
            int n = (Integer) v[0];
            byte[] in = new byte[n];
            for (int i = 0; i < n; i++) in[i] = (byte) (i % 251);
            String got = hex(in);
            boolean ok = got.equals(v[1]);
            if (!ok) failures++;
            if (verbose || !ok) System.out.println((ok ? "ok   " : "FAIL ") + "blake3 len=" + n + " " + got);
        }
        return failures;
    }
}
