package com.kriskowal.yay;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.math.BigInteger;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.*;
import java.util.stream.Stream;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;

/**
 * Tagged values. Reads the design's cases in doc/design/tagged-values-g/cases/: each .yay parses
 * to its .json, with a tagged value as {"#tag": tag, "payload": value} and bytes as "*hex"; each
 * .nay and .meh is refused as YAY.
 */
class TaggedTest {

  private static final Path CASES = Paths.get("..", "doc", "design", "tagged-values-g", "cases");

  @TestFactory
  Stream<DynamicTest> testCasesAccepted() throws IOException {
    return Files.list(CASES)
        .filter(p -> p.toString().endsWith(".yay"))
        .sorted()
        .map(
            path ->
                DynamicTest.dynamicTest(
                    path.getFileName().toString(),
                    () -> {
                      String name = path.getFileName().toString();
                      Path json = path.resolveSibling(name.replace(".yay", ".json"));
                      Object expected = new Json(Files.readString(json)).value();
                      Object actual = Yay.parse(Files.readString(path), name);
                      assertTrue(
                          Tagged.valuesEqual(expected, actual),
                          name + "\nExpected: " + expected + "\nActual: " + actual);
                    }));
  }

  @TestFactory
  Stream<DynamicTest> testCasesRefused() throws IOException {
    return Files.list(CASES)
        .filter(p -> p.toString().endsWith(".nay") || p.toString().endsWith(".meh"))
        .sorted()
        .map(
            path ->
                DynamicTest.dynamicTest(
                    path.getFileName().toString(),
                    () -> {
                      String source = Files.readString(path);
                      assertThrows(
                          Yay.YayException.class,
                          () -> Yay.parse(source, path.getFileName().toString()));
                    }));
  }

  @Test
  void testModel() {
    Tagged a = new Tagged("t", BigInteger.ONE);
    assertEquals("t", a.tag());
    assertEquals(BigInteger.ONE, a.payload());
    assertEquals(a, new Tagged("t", BigInteger.ONE));
    assertEquals(a.hashCode(), new Tagged("t", BigInteger.ONE).hashCode());
    assertNotEquals(a, new Tagged("u", BigInteger.ONE));
    assertNotEquals(a, new Tagged("t", BigInteger.TWO));
    assertNotEquals(a, BigInteger.ONE);
    assertNotEquals(BigInteger.ONE, a);
    assertEquals(new Tagged("b", new byte[] {1, 2}), new Tagged("b", new byte[] {1, 2}));
    assertNotEquals(new Tagged("b", new byte[] {1, 2}), new Tagged("b", new byte[] {1, 3}));
    assertThrows(NullPointerException.class, () -> new Tagged(null, 1));
  }

  @Test
  void testEscapesAndNesting() {
    Map<String, Object> want = new LinkedHashMap<>();
    want.put("a", new Tagged("outer", new Tagged("xA\\\n", BigInteger.ONE)));
    assertEquals(want, Yay.parse("a: 1 (x\\u{41}\\\\\\n) (outer)\n"));

    want = new LinkedHashMap<>();
    want.put("a", new Tagged("t", BigInteger.ONE));
    assertEquals(want, Yay.parse("a: 1 (t)  # note\n"));

    Map<String, Object> inner = new LinkedHashMap<>();
    inner.put("k", "v");
    assertEquals(
        List.of(new Tagged("a", List.of(new Tagged("b", inner)))),
        Yay.parse("- (a)\n  - (b)\n    k: 'v'\n"));
  }

  @Test
  void testRefused() {
    for (String source :
        new String[] {
          "a: 1 (x\\\"y)\n", "a: 1 (x\\(y)\n", "a: 1 (x\n", "a: 1  (t)\n", "a: 1 (t)  b\n",
          "a: (t)\n", "- (t) 1\n", "(t)\n1\n", "a: [1(t)]\n", "a: {b: 'x'(t)}\n",
        }) {
      assertThrows(Yay.YayException.class, () -> Yay.parse(source), source);
    }
  }

  /** A small JSON reader for the expected values, desugaring tags and bytes. */
  private static final class Json {
    private final String s;
    private int i;

    Json(String s) {
      this.s = s;
    }

    private void space() {
      while (i < s.length() && Character.isWhitespace(s.charAt(i))) i++;
    }

    Object value() {
      space();
      char c = s.charAt(i);
      if (c == '{') {
        i++;
        Map<String, Object> obj = new LinkedHashMap<>();
        space();
        while (s.charAt(i) != '}') {
          String key = string();
          space();
          i++; // :
          obj.put(key, value());
          space();
          if (s.charAt(i) == ',') i++;
          space();
        }
        i++;
        if (obj.size() == 2 && obj.containsKey("#tag") && obj.containsKey("payload")) {
          return new Tagged((String) obj.get("#tag"), obj.get("payload"));
        }
        return obj;
      }
      if (c == '[') {
        i++;
        List<Object> list = new ArrayList<>();
        space();
        while (s.charAt(i) != ']') {
          list.add(value());
          space();
          if (s.charAt(i) == ',') i++;
          space();
        }
        i++;
        return list;
      }
      if (c == '"') {
        String str = string();
        if (str.startsWith("*")) return HexFormat.of().parseHex(str.substring(1));
        // The cases' JSON spells NaN as a string
        if (str.equals("NaN")) return Double.NaN;
        return str;
      }
      for (String word : new String[] {"null", "true", "false", "-Infinity", "Infinity"}) {
        if (s.startsWith(word, i)) {
          i += word.length();
          switch (word) {
            case "null":
              return null;
            case "true":
              return Boolean.TRUE;
            case "false":
              return Boolean.FALSE;
            case "-Infinity":
              return Double.NEGATIVE_INFINITY;
            default:
              return Double.POSITIVE_INFINITY;
          }
        }
      }
      int start = i;
      while (i < s.length() && "+-0123456789.eE".indexOf(s.charAt(i)) >= 0) i++;
      String num = s.substring(start, i);
      if (num.contains(".") || num.contains("e") || num.contains("E")) {
        return Double.parseDouble(num);
      }
      return new BigInteger(num);
    }

    private String string() {
      StringBuilder out = new StringBuilder();
      i++;
      while (s.charAt(i) != '"') {
        char c = s.charAt(i++);
        if (c != '\\') {
          out.append(c);
          continue;
        }
        char e = s.charAt(i++);
        switch (e) {
          case 'n':
            out.append('\n');
            break;
          case 't':
            out.append('\t');
            break;
          case 'u':
            out.append((char) Integer.parseInt(s.substring(i, i + 4), 16));
            i += 4;
            break;
          default:
            out.append(e);
        }
      }
      i++;
      return out.toString();
    }
  }
}
