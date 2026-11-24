package com.kriskowal.yay;

import java.util.Arrays;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * A tagged value, {@code 'X-1' (com.example/sku)} or a block below {@code key: (tag)}: a tag, a
 * string, and a payload, any value. Tagged values are equal when their tags and payloads are; a
 * tagged value never equals its payload. YAY gives no tag a meaning.
 */
public record Tagged(String tag, Object payload) {

  public Tagged {
    Objects.requireNonNull(tag, "tag");
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof Tagged that
        && tag.equals(that.tag)
        && valuesEqual(payload, that.payload);
  }

  @Override
  public int hashCode() {
    return 31 * tag.hashCode() + valueHash(payload);
  }

  @Override
  public String toString() {
    return "Tagged[tag=" + tag + ", payload=" + valueString(payload) + "]";
  }

  /** Equality over YAY values, with byte arrays compared by content. */
  static boolean valuesEqual(Object a, Object b) {
    if (a instanceof byte[] x && b instanceof byte[] y) {
      return Arrays.equals(x, y);
    }
    if (a instanceof List<?> x && b instanceof List<?> y) {
      if (x.size() != y.size()) return false;
      Iterator<?> i = x.iterator();
      Iterator<?> j = y.iterator();
      while (i.hasNext()) {
        if (!valuesEqual(i.next(), j.next())) return false;
      }
      return true;
    }
    if (a instanceof Map<?, ?> x && b instanceof Map<?, ?> y) {
      if (x.size() != y.size()) return false;
      for (Map.Entry<?, ?> e : x.entrySet()) {
        if (!y.containsKey(e.getKey()) || !valuesEqual(e.getValue(), y.get(e.getKey()))) {
          return false;
        }
      }
      return true;
    }
    return Objects.equals(a, b);
  }

  private static int valueHash(Object v) {
    if (v instanceof byte[] b) return Arrays.hashCode(b);
    if (v instanceof List<?> list) {
      int h = 1;
      for (Object item : list) h = 31 * h + valueHash(item);
      return h;
    }
    if (v instanceof Map<?, ?> map) {
      int h = 0;
      for (Map.Entry<?, ?> e : map.entrySet()) {
        h += Objects.hashCode(e.getKey()) ^ valueHash(e.getValue());
      }
      return h;
    }
    return Objects.hashCode(v);
  }

  private static String valueString(Object v) {
    return v instanceof byte[] b ? "<" + java.util.HexFormat.of().formatHex(b) + ">" : String.valueOf(v);
  }
}
