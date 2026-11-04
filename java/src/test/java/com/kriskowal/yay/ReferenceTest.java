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
 * References. Reads the design's cases in doc/design/references/cases/: each .yay parses to its
 * .yson, with a reference spelled "$uri", and each .nay and .meh is refused, MEH's leniency not
 * being YAY's.
 */
class ReferenceTest {

  private static final Path CASES = Paths.get("..", "doc", "design", "references", "cases");

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
                      Path yson = path.resolveSibling(name.replace(".yay", ".yson"));
                      Object expected = new Json(Files.readString(yson)).value();
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
    Reference a = new Reference("https://x");
    assertEquals("https://x", a.uri());
    assertEquals(a, new Reference("https://x"));
    assertEquals(a.hashCode(), new Reference("https://x").hashCode());
    assertEquals("<https://x>", a.toString());
    assertNotEquals(a, new Reference("https://x/"));
    // By text, so percent-encodings are not normalized
    assertNotEquals(new Reference("a:%2f"), new Reference("a:%2F"));
    // Never a string, nor a tagged value
    assertNotEquals(a, "https://x");
    assertNotEquals("https://x", a);
    assertNotEquals(a, new Tagged("https://x", null));
    assertThrows(NullPointerException.class, () -> new Reference(null));
  }

  @Test
  void testRefused() {
    // The Rust, JavaScript, Python and Go ports' messages and columns
    String[][] cases = {
      {"a: <HTTPS://x>\n", "Uppercase scheme \"HTTPS\" in reference (use lowercase) at 1:4 of <t>"},
      {"a: <https://x y>\n", "Unexpected space in reference at 1:14 of <t>"},
      {"a: <https://x\"y>\n", "Unexpected character \"\"\" in reference at 1:14 of <t>"},
      {
        "a: <https://x/%2g>\n",
        "Bad percent-encoding in reference (use % and two hex digits) at 1:15 of <t>"
      },
      {
        "a: <https://x/%2>\n",
        "Bad percent-encoding in reference (use % and two hex digits) at 1:15 of <t>"
      },
      {"a: <https://x\n", "Unmatched angle bracket at 1:4 of <t>"},
      {"a: <https://x>(t)\n", "Expected one space before a tag at 1:15 of <t>"},
      {"a: <https://x>rgb\n", "Unexpected character \"r\" at 1:15 of <t>"},
      {"<https://x>: 1\n", "Invalid key at 1:1 of <t>"},
    };
    for (String[] c : cases) {
      Yay.YayException e = assertThrows(Yay.YayException.class, () -> Yay.parse(c[0], "t"), c[0]);
      assertEquals(c[1], e.getMessage(), c[0]);
    }
  }

  @Test
  void testParse() {
    assertEquals(
        new Tagged("Counter", new Reference("ocapn://a.tcp")),
        Yay.parse("<ocapn://a.tcp> (Counter)\n"));

    // Unchanged in strings and tags, and bytes without a scheme
    Object v = Yay.parse("a: '<https://x>' (<b:c>)\nb: <ff>\n");
    Map<String, Object> want = new LinkedHashMap<>();
    want.put("a", new Tagged("<b:c>", "<https://x>"));
    want.put("b", new byte[] {(byte) 0xff});
    assertTrue(Tagged.valuesEqual(want, v), String.valueOf(v));

    // A reference's punctuation is its own
    want = new LinkedHashMap<>();
    want.put("a", List.of(new Reference("x:a,b]c#d"), BigInteger.ONE));
    assertEquals(want, Yay.parse("a: [<x:a,b]c#d>, 1]  # c\n"));

    want = new LinkedHashMap<>();
    want.put("a", new Tagged("u", Map.of("b", new Reference("x:(t)"))));
    assertEquals(want, Yay.parse("a: {b: <x:(t)>} (u)\n"));
  }

  /**
   * A small JSON reader for the expected values, desugaring YSON's strings: "$uri" a reference,
   * "*hex" bytes, "+n" and "-n" integers, "#nan" and the infinities doubles, and "!" before text
   * that would read as one of those.
   */
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
          obj.put(key.startsWith("!") ? key.substring(1) : key, value());
          space();
          if (s.charAt(i) == ',') i++;
          space();
        }
        i++;
        if (obj.size() == 2 && obj.containsKey("#tag") && obj.containsKey("payload")) {
          String tag = (String) obj.get("#tag");
          return new Tagged(tag.startsWith("!") ? tag.substring(1) : tag, obj.get("payload"));
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
        return ysonString(string());
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

    private static Object ysonString(String str) {
      if (str.isEmpty()) return str;
      switch (str.charAt(0)) {
        case '$':
          return new Reference(str.substring(1));
        case '*':
          return HexFormat.of().parseHex(str.substring(1));
        case '+':
          return new BigInteger(str.substring(1));
        case '-':
          return new BigInteger(str);
        case '!':
          return str.substring(1);
        default:
          break;
      }
      switch (str) {
        case "#nan":
          return Double.NaN;
        case "#+inf":
          return Double.POSITIVE_INFINITY;
        case "#-inf":
          return Double.NEGATIVE_INFINITY;
        default:
          return str;
      }
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
