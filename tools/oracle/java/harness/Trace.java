package harness;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** One scenario's output: {@code expected.jsonl} lines plus named sidecar files. */
final class Trace {
    final List<String> lines = new ArrayList<>();
    final Map<String, byte[]> sidecars = new LinkedHashMap<>();

    void add(Json.Line line) {
        lines.add(line.toString());
    }

    /** Writes {@code expected.jsonl} (UTF-8, every line ends in "\n") and the sidecars into {@code dir}. */
    void write(Path dir) throws IOException {
        Files.createDirectories(dir);
        StringBuilder b = new StringBuilder();
        for (String l : lines) b.append(l).append('\n');
        Files.write(dir.resolve("expected.jsonl"), b.toString().getBytes(StandardCharsets.UTF_8));
        for (Map.Entry<String, byte[]> e : sidecars.entrySet()) {
            Files.write(dir.resolve(e.getKey()), e.getValue());
        }
    }
}
