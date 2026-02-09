package com.kriskowal.yay;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.math.BigInteger;
import java.nio.file.*;
import java.util.*;
import java.util.stream.Stream;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;

public class YayTest {

  private static final Path TEST_ROOT = Paths.get("../test");
  private static final Path YAY_DIR = TEST_ROOT.resolve("yay");
  private static final Path NAY_DIR = TEST_ROOT.resolve("nay");

  @Test
  void testNull() {
    assertNull(Yay.parse("null"));
  }

  @Test
  void testBooleans() {
    assertEquals(Boolean.TRUE, Yay.parse("true"));
    assertEquals(Boolean.FALSE, Yay.parse("false"));
  }

  @Test
  void testIntegers() {
    assertEquals(new BigInteger("42"), Yay.parse("42"));
    assertEquals(new BigInteger("-123"), Yay.parse("-123"));
    assertEquals(new BigInteger("1000000"), Yay.parse("1_000_000"));
  }

  @Test
  void testFloats() {
    assertEquals(3.14, Yay.parse("3.14"));
    assertEquals(Double.POSITIVE_INFINITY, Yay.parse("infinity"));
    assertEquals(Double.NEGATIVE_INFINITY, Yay.parse("-infinity"));
    assertTrue(Double.isNaN((Double) Yay.parse("nan")));
  }

  @Test
  void testStrings() {
    assertEquals("hello", Yay.parse("\"hello\""));
    assertEquals("world", Yay.parse("'world'"));
    assertEquals("\"\\//\b\f\n\r\t\u263A", Yay.parse("\"\\\"\\\\\\//\\b\\f\\n\\r\\t\\u{263A}\""));
  }

  @Test
  void testInlineArray() {
    @SuppressWarnings("unchecked")
    List<Object> arr = (List<Object>) Yay.parse("[1, 2, 3]");
    assertEquals(3, arr.size());
    assertEquals(new BigInteger("1"), arr.get(0));
    assertEquals(new BigInteger("2"), arr.get(1));
    assertEquals(new BigInteger("3"), arr.get(2));
  }

  @Test
  void testInlineObject() {
    @SuppressWarnings("unchecked")
    Map<String, Object> obj = (Map<String, Object>) Yay.parse("{a: 1, b: 2}");
    assertEquals(new BigInteger("1"), obj.get("a"));
    assertEquals(new BigInteger("2"), obj.get("b"));
  }

  @Test
  void testBytes() {
    byte[] bytes = (byte[]) Yay.parse("<cafe>");
    assertArrayEquals(new byte[] {(byte) 0xca, (byte) 0xfe}, bytes);
  }

  @TestFactory
  Stream<DynamicTest> testAllYayFixtures() throws IOException {
    if (!Files.exists(YAY_DIR)) {
      return Stream.empty();
    }

    return Files.list(YAY_DIR)
        .filter(p -> p.toString().endsWith(".yay"))
        .sorted()
        .map(
            yayPath -> {
              String name = yayPath.getFileName().toString().replace(".yay", "");

              return DynamicTest.dynamicTest(
                  name,
                  () -> {
                    // Check if we have an expected value for this fixture
                    if (!FixturesGen.FIXTURES.containsKey(name)) {
                      System.out.println("SKIP: " + name + " (no expected value in FixturesGen)");
                      return;
                    }

                    String yayContent = Files.readString(yayPath);
                    Object expected = FixturesGen.FIXTURES.get(name);

                    try {
                      Object result = Yay.parse(yayContent, name + ".yay");

                      // Use deep equality comparison
                      assertTrue(
                          deepEqual(result, expected),
                          "Mismatch for "
                              + name
                              + "\nExpected: "
                              + formatValue(expected)
                              + "\nActual: "
                              + formatValue(result));
                    } catch (Yay.YayException e) {
                      fail("Unexpected parse error for " + name + ": " + e.getMessage());
                    }
                  });
            });
  }

  @TestFactory
  Stream<DynamicTest> testAllNayFixtures() throws IOException {
    if (!Files.exists(NAY_DIR)) {
      return Stream.empty();
    }

    return Files.list(NAY_DIR)
        .filter(p -> p.toString().endsWith(".nay"))
        .sorted()
        .map(
            nayPath -> {
              String name = nayPath.getFileName().toString().replace(".nay", "");
              Path errorPath = nayPath.resolveSibling(name + ".error");

              return DynamicTest.dynamicTest(
                  name,
                  () -> {
                    String nayContent;
                    try {
                      nayContent = Files.readString(nayPath);
                    } catch (IOException e) {
                      fail("Cannot read file: " + name);
                      return;
                    }

                    String expectedError = "";
                    if (Files.exists(errorPath)) {
                      expectedError = Files.readString(errorPath).trim();
                    }

                    try {
                      Object result = Yay.parse(nayContent, name + ".nay");
                      fail(
                          "Expected error for "
                              + name
                              + " but parsing succeeded with: "
                              + formatValue(result));
                    } catch (Yay.YayException e) {
                      // Expected - parsing should fail
                      String actualError = e.getMessage();
                      if (!expectedError.isEmpty()) {
                        assertTrue(
                            actualError.contains(expectedError),
                            "Error mismatch for "
                                + name
                                + "\nExpected to contain: "
                                + expectedError
                                + "\nActual: "
                                + actualError);
                      }
                    }
                  });
            });
  }

  // ========================================================================
  // Deep Equality Comparison
  // ========================================================================

  private boolean deepEqual(Object a, Object b) {
    // Handle null
    if (a == null && b == null) return true;
    if (a == null || b == null) return false;

    // Handle NaN
    if (a instanceof Double && b instanceof Double) {
      Double da = (Double) a;
      Double db = (Double) b;
      if (da.isNaN() && db.isNaN()) return true;
      // Handle -0.0
      if (da.equals(0.0) && db.equals(0.0)) {
        return Double.doubleToRawLongBits(da) == Double.doubleToRawLongBits(db);
      }
      return da.equals(db);
    }

    // Handle BigInteger
    if (a instanceof BigInteger && b instanceof BigInteger) {
      return ((BigInteger) a).compareTo((BigInteger) b) == 0;
    }

    // Handle byte[]
    if (a instanceof byte[] && b instanceof byte[]) {
      return Arrays.equals((byte[]) a, (byte[]) b);
    }

    // Handle List
    if (a instanceof List && b instanceof List) {
      List<?> la = (List<?>) a;
      List<?> lb = (List<?>) b;
      if (la.size() != lb.size()) return false;
      for (int i = 0; i < la.size(); i++) {
        if (!deepEqual(la.get(i), lb.get(i))) return false;
      }
      return true;
    }

    // Handle Map
    if (a instanceof Map && b instanceof Map) {
      Map<?, ?> ma = (Map<?, ?>) a;
      Map<?, ?> mb = (Map<?, ?>) b;
      if (ma.size() != mb.size()) return false;
      for (Object key : ma.keySet()) {
        if (!mb.containsKey(key)) return false;
        if (!deepEqual(ma.get(key), mb.get(key))) return false;
      }
      return true;
    }

    // Default comparison
    return a.equals(b);
  }

  // ========================================================================
  // Value Formatting for Error Messages
  // ========================================================================

  private String formatValue(Object value) {
    if (value == null) {
      return "null";
    }
    if (value instanceof String) {
      return "\"" + escapeString((String) value) + "\"";
    }
    if (value instanceof byte[]) {
      byte[] bytes = (byte[]) value;
      StringBuilder sb = new StringBuilder("bytes(");
      for (int i = 0; i < bytes.length; i++) {
        if (i > 0) sb.append(", ");
        sb.append(String.format("0x%02x", bytes[i] & 0xFF));
      }
      sb.append(")");
      return sb.toString();
    }
    if (value instanceof List) {
      List<?> list = (List<?>) value;
      StringBuilder sb = new StringBuilder("list(");
      for (int i = 0; i < list.size(); i++) {
        if (i > 0) sb.append(", ");
        sb.append(formatValue(list.get(i)));
      }
      sb.append(")");
      return sb.toString();
    }
    if (value instanceof Map) {
      Map<?, ?> map = (Map<?, ?>) value;
      StringBuilder sb = new StringBuilder("map(");
      int i = 0;
      for (Map.Entry<?, ?> entry : map.entrySet()) {
        if (i > 0) sb.append(", ");
        sb.append(formatValue(entry.getKey())).append(", ").append(formatValue(entry.getValue()));
        i++;
      }
      sb.append(")");
      return sb.toString();
    }
    if (value instanceof Double) {
      Double d = (Double) value;
      if (d.isNaN()) return "NaN";
      if (d.isInfinite()) return d > 0 ? "Infinity" : "-Infinity";
    }
    return value.toString();
  }

  private String escapeString(String s) {
    StringBuilder sb = new StringBuilder();
    for (int i = 0; i < s.length(); i++) {
      char c = s.charAt(i);
      switch (c) {
        case '"':
          sb.append("\\\"");
          break;
        case '\\':
          sb.append("\\\\");
          break;
        case '\n':
          sb.append("\\n");
          break;
        case '\r':
          sb.append("\\r");
          break;
        case '\t':
          sb.append("\\t");
          break;
        case '\b':
          sb.append("\\b");
          break;
        case '\f':
          sb.append("\\f");
          break;
        default:
          if (c < 0x20) {
            sb.append(String.format("\\u%04x", (int) c));
          } else {
            sb.append(c);
          }
      }
    }
    return sb.toString();
  }
}
