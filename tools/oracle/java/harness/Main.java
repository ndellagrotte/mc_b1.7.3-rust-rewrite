package harness;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;

/**
 * Golden-fixture oracle (PLAN_v4 §4, tests/golden/README.md).
 *
 * <pre>
 * java -cp "build/oracle:$CP" harness.Main &lt;scenarios-dir&gt; &lt;out-dir&gt; [area/name ...]
 * java -cp "build/oracle:$CP" harness.Main --selftest
 * </pre>
 *
 * Walks {@code <scenarios-dir>/<area>/<name>/scenario.json} in sorted order and
 * writes {@code <out-dir>/<area>/<name>/expected.jsonl} (+ sidecars). Optional
 * trailing arguments restrict the run to the named scenarios.
 *
 * <p>{@code -Dharness.perturbSeed=<long>} XORs every scenario seed with the
 * value (sensitivity check, PLAN_v4 §4.4 item 2).
 */
public final class Main {
    static final long PERTURB_SEED = Long.getLong("harness.perturbSeed", 0L);

    private Main() {}

    public static void main(String[] args) throws Exception {
        if (args.length == 1 && args[0].equals("--selftest")) {
            System.exit(Blake3.selfTest(true) == 0 ? 0 : 1);
        }
        if (args.length < 2) {
            System.err.println("usage: harness.Main <scenarios-dir> <out-dir> [area/name ...] | --selftest");
            System.exit(2);
        }
        if (Blake3.selfTest(false) != 0) {
            System.err.println("harness: BLAKE3 self-test failed");
            System.exit(1);
        }
        Path in = Paths.get(args[0]);
        Path out = Paths.get(args[1]);
        List<String> only = Arrays.asList(args).subList(2, args.length);

        List<Path> scenarios = new ArrayList<>();
        try (Stream<Path> s = Files.walk(in, 3)) {
            s.filter(p -> p.getFileName().toString().equals("scenario.json"))
             .filter(p -> in.relativize(p).getNameCount() == 3)
             .sorted()
             .forEach(scenarios::add);
        }
        int ran = 0;
        for (Path file : scenarios) {
            Path rel = in.relativize(file.getParent());
            String id = rel.toString().replace('\\', '/');
            if (!only.isEmpty() && !only.contains(id)) continue;
            try {
                Map<String, Object> sc = Scenario.load(file, rel.getFileName().toString());
                Trace t = TierA.run(sc);
                t.write(out.resolve(rel));
                System.out.println("harness: " + id + " (" + t.lines.size() + " lines)");
                ran++;
            } catch (Exception e) {
                System.err.println("harness: scenario " + id + " failed");
                e.printStackTrace();
                System.exit(1);
            }
        }
        if (ran == 0) {
            System.err.println("harness: no scenarios found under " + in);
            System.exit(1);
        }
    }

    /** Typed access to a parsed scenario.json. */
    static final class Scenario {
        private Scenario() {}

        @SuppressWarnings("unchecked")
        static Map<String, Object> load(Path file, String dirName) throws Exception {
            String text = new String(Files.readAllBytes(file), StandardCharsets.UTF_8);
            Map<String, Object> sc = (Map<String, Object>) Json.parse(text);
            if (!dirName.equals(sc.get("name"))) {
                throw new IllegalArgumentException("\"name\" must equal the directory name " + dirName);
            }
            if (!"A".equals(sc.get("tier"))) {
                throw new IllegalArgumentException("only tier \"A\" is implemented, got " + sc.get("tier"));
            }
            return sc;
        }

        static String str(Map<String, Object> m, String k) {
            Object v = m.get(k);
            if (!(v instanceof String)) throw new IllegalArgumentException("missing string field \"" + k + "\"");
            return (String) v;
        }

        static long num(Map<String, Object> m, String k) {
            Object v = m.get(k);
            if (!(v instanceof Long)) throw new IllegalArgumentException("missing integer field \"" + k + "\"");
            return (Long) v;
        }

        static boolean flag(Map<String, Object> m, String k) {
            return Boolean.TRUE.equals(m.get(k));
        }

        @SuppressWarnings("unchecked")
        static List<Object> list(Map<String, Object> m, String k) {
            Object v = m.get(k);
            if (!(v instanceof List)) throw new IllegalArgumentException("missing array field \"" + k + "\"");
            return (List<Object>) v;
        }

        /** The scenario seed with {@code -Dharness.perturbSeed} applied. */
        static long seed(Map<String, Object> m) {
            return num(m, "seed") ^ PERTURB_SEED;
        }
    }
}
