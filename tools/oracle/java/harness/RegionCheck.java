package harness;

import java.io.ByteArrayInputStream;
import java.io.DataInputStream;
import java.io.File;
import net.minecraft.src.CompressedStreamTools;
import net.minecraft.src.NBTTagCompound;
import net.minecraft.src.RegionFile;

/**
 * Cross-read check for the Rust McRegion writer (WP-1.9a; not a fixture kind).
 *
 * <pre>
 * java -cp "build/oracle:$CP" harness.RegionCheck &lt;file.mcr&gt;
 * </pre>
 *
 * Opens the file with the vanilla {@code RegionFile} (RegionFile.java:24-85; the
 * constructor may rewrite the file, so pass a copy) and prints one JSON line per chunk
 * whose offset entry is non-zero ({@code func_22202_c}, RegionFile.java:255-257), in
 * index order {@code x + z * 32}: {@code {"x","z","len","blake3","root"}} with the
 * uncompressed NBT bytes read through {@code getChunkDataInputStream}
 * (RegionFile.java:112-159), their BLAKE3, and the canonical tree of those bytes parsed
 * by {@code CompressedStreamTools.read} (tests/golden/README.md, "Canonical tree"), or
 * {@code {"x","z","null":true}} when the vanilla reader returns null. Run by
 * {@code crates/mc-sim/tests/region_file.rs} when the oracle is built.
 */
public final class RegionCheck {
    private RegionCheck() {}

    public static void main(String[] args) throws Exception {
        if (args.length != 1) {
            System.err.println("usage: harness.RegionCheck <file.mcr>");
            System.exit(2);
        }
        RegionFile region = new RegionFile(new File(args[0]));
        StringBuilder out = new StringBuilder();
        for (int i = 0; i < 1024; i++) {
            int x = i & 31, z = i >> 5;
            if (!region.func_22202_c(x, z)) continue;
            Json.Line line = new Json.Line().put("x", x).put("z", z);
            DataInputStream in = region.getChunkDataInputStream(x, z);
            if (in == null) {
                out.append(line.put("null", true)).append('\n');
                continue;
            }
            byte[] bytes;
            try {
                bytes = in.readAllBytes();
            } finally {
                in.close();
            }
            NBTTagCompound root = CompressedStreamTools.read(new DataInputStream(new ByteArrayInputStream(bytes)));
            line.put("len", bytes.length).put("blake3", Blake3.hex(bytes)).raw("root", Nbt.canonicalRoot(root));
            out.append(line).append('\n');
        }
        region.func_22196_b();
        System.out.print(out);
    }
}
