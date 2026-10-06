package harness;

import java.lang.reflect.Field;
import java.util.HashMap;
import java.util.Map;

/** Reflective access to vanilla private/protected fields (the harness never edits vanilla code). */
final class Reflect {
    private static final Map<String, Field> CACHE = new HashMap<>();

    private Reflect() {}

    static Field field(Class<?> owner, String name) {
        String key = owner.getName() + "#" + name;
        Field f = CACHE.get(key);
        if (f == null) {
            try {
                f = owner.getDeclaredField(name);
            } catch (NoSuchFieldException e) {
                throw new IllegalStateException("vanilla field " + key + " not found", e);
            }
            f.setAccessible(true);
            CACHE.put(key, f);
        }
        return f;
    }

    static Object get(Class<?> owner, String name, Object target) {
        try {
            return field(owner, name).get(target);
        } catch (IllegalAccessException e) {
            throw new IllegalStateException(e);
        }
    }

    static int getInt(Class<?> owner, String name, Object target) {
        try {
            return field(owner, name).getInt(target);
        } catch (IllegalAccessException e) {
            throw new IllegalStateException(e);
        }
    }

    static float getFloat(Class<?> owner, String name, Object target) {
        try {
            return field(owner, name).getFloat(target);
        } catch (IllegalAccessException e) {
            throw new IllegalStateException(e);
        }
    }

    static boolean getBoolean(Class<?> owner, String name, Object target) {
        try {
            return field(owner, name).getBoolean(target);
        } catch (IllegalAccessException e) {
            throw new IllegalStateException(e);
        }
    }

    static void set(Class<?> owner, String name, Object target, Object value) {
        try {
            Field f = field(owner, name);
            if (f.getType() == int.class) f.setInt(target, (Integer) value);
            else if (f.getType() == long.class) f.setLong(target, (Long) value);
            else if (f.getType() == float.class) f.setFloat(target, (Float) value);
            else f.set(target, value);
        } catch (IllegalAccessException e) {
            throw new IllegalStateException(e);
        }
    }
}
