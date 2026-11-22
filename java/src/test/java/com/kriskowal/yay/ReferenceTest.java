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
 * References. MEH's leniency is not YAY's, so the corpus's MEH reference cases,
 * test/meh/reference-*.meh, are refused as YAY. The corpus reference cases in test/yay and
 * test/nay are read with the rest of the corpus.
 */
class ReferenceTest {

  private static final Path MEH = Paths.get("..", "test", "meh");

  @TestFactory
  Stream<DynamicTest> testMehCasesRefused() throws IOException {
    List<Path> paths;
    try (Stream<Path> list = Files.list(MEH)) {
      paths =
          list.filter(
                  p -> {
                    String name = p.getFileName().toString();
                    return name.startsWith("reference-") && name.endsWith(".meh");
                  })
              .sorted()
              .toList();
    }
    assertTrue(paths.size() >= 2, "MEH reference cases in " + MEH);
    return paths.stream()
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
  void testMadeOnlyFromReferenceText() {
    for (String uri :
        List.of(
            "", "not a uri", "HTTPS://x", "urn:a b", "https://a>b", "https://x/%zz", "./a.yay")) {
      IllegalArgumentException e =
          assertThrows(IllegalArgumentException.class, () -> new Reference(uri), uri);
      assertTrue(e.getMessage().startsWith("Not a reference: "), e.getMessage());
    }
    for (String uri : List.of("urn:x", "https://example.com/a?b=c#d", "urn:%C3%A9", "urn:\u00e9")) {
      assertEquals(uri, new Reference(uri).uri());
      assertEquals(new Reference(uri), Yay.parse("<" + uri + ">\n"));
    }
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
      {"<https://x>: 1\n", "Unexpected character \":\" at 1:12 of <t>"},
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
}
