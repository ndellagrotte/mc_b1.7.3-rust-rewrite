package harness;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Minimal JSON for the harness.
 *
 * <p>Reading: objects become {@code LinkedHashMap}, arrays {@code ArrayList},
 * integers {@code Long} (full signed 64-bit range, parsed exactly), strings,
 * booleans, null. Non-integer numbers are rejected: scenarios carry floats as
 * bit-pattern strings only.
 *
 * <p>Writing: {@link Line} builds one trace object with keys in insertion
 * order; floats only through {@link Line#f32}/{@link Line#f64}.
 */
final class Json {
    private final String s;
    private int p;

    private Json(String s) {
        this.s = s;
    }

    static Object parse(String text) {
        Json j = new Json(text);
        Object v = j.value();
        j.ws();
        if (j.p != j.s.length()) throw j.err("trailing data");
        return v;
    }

    private Object value() {
        ws();
        if (p >= s.length()) throw err("unexpected end");
        char c = s.charAt(p);
        switch (c) {
            case '{': return object();
            case '[': return array();
            case '"': return string();
            case 't': return literal("true", Boolean.TRUE);
            case 'f': return literal("false", Boolean.FALSE);
            case 'n': return literal("null", null);
            default: return number();
        }
    }

    private Map<String, Object> object() {
        Map<String, Object> m = new LinkedHashMap<>();
        p++;
        ws();
        if (peek() == '}') { p++; return m; }
        while (true) {
            ws();
            if (peek() != '"') throw err("expected key");
            String k = string();
            ws();
            expect(':');
            if (m.put(k, value()) != null) throw err("duplicate key " + k);
            ws();
            char c = s.charAt(p++);
            if (c == '}') return m;
            if (c != ',') throw err("expected , or }");
        }
    }

    private List<Object> array() {
        List<Object> a = new ArrayList<>();
        p++;
        ws();
        if (peek() == ']') { p++; return a; }
        while (true) {
            a.add(value());
            ws();
            char c = s.charAt(p++);
            if (c == ']') return a;
            if (c != ',') throw err("expected , or ]");
        }
    }

    private String string() {
        expect('"');
        StringBuilder b = new StringBuilder();
        while (true) {
            char c = s.charAt(p++);
            if (c == '"') return b.toString();
            if (c != '\\') { b.append(c); continue; }
            char e = s.charAt(p++);
            switch (e) {
                case '"': case '\\': case '/': b.append(e); break;
                case 'b': b.append('\b'); break;
                case 'f': b.append('\f'); break;
                case 'n': b.append('\n'); break;
                case 'r': b.append('\r'); break;
                case 't': b.append('\t'); break;
                case 'u': b.append((char) Integer.parseInt(s.substring(p, p + 4), 16)); p += 4; break;
                default: throw err("bad escape");
            }
        }
    }

    private Long number() {
        int start = p;
        if (peek() == '-') p++;
        while (p < s.length() && Character.isDigit(s.charAt(p))) p++;
        if (p < s.length() && ".eE".indexOf(s.charAt(p)) >= 0) {
            throw err("non-integer number; encode floats as bit-pattern strings");
        }
        try {
            return Long.parseLong(s.substring(start, p));
        } catch (NumberFormatException e) {
            throw err("bad integer " + s.substring(start, p));
        }
    }

    private Object literal(String word, Object v) {
        if (!s.startsWith(word, p)) throw err("bad literal");
        p += word.length();
        return v;
    }

    private void ws() {
        while (p < s.length() && " \t\r\n".indexOf(s.charAt(p)) >= 0) p++;
    }

    private char peek() {
        return p < s.length() ? s.charAt(p) : '\0';
    }

    private void expect(char c) {
        if (peek() != c) throw err("expected '" + c + "'");
        p++;
    }

    private IllegalArgumentException err(String msg) {
        return new IllegalArgumentException("JSON: " + msg + " at offset " + p);
    }

    /** One trace line: a flat JSON object, keys in insertion order. */
    static final class Line {
        private final StringBuilder b = new StringBuilder("{");

        private Line key(String k) {
            if (b.length() > 1) b.append(',');
            quote(k);
            b.append(':');
            return this;
        }

        Line put(String k, long v) {
            key(k).b.append(v);
            return this;
        }

        Line put(String k, boolean v) {
            key(k).b.append(v);
            return this;
        }

        Line put(String k, String v) {
            key(k).quote(v);
            return this;
        }

        Line f32(String k, float v) {
            return put(k, Hex.f32(v));
        }

        Line f64(String k, double v) {
            return put(k, Hex.f64(v));
        }

        private void quote(String v) {
            b.append('"');
            for (int i = 0; i < v.length(); i++) {
                char c = v.charAt(i);
                if (c == '"' || c == '\\') b.append('\\').append(c);
                else if (c < 0x20) b.append(String.format("\\u%04x", (int) c));
                else b.append(c);
            }
            b.append('"');
        }

        @Override
        public String toString() {
            return b + "}";
        }
    }
}
