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
 * Tagged values. MEH's leniency is not YAY's, so the corpus's MEH tagged cases,
 * test/meh/tagged-*.meh, are refused as YAY. The corpus tagged cases in test/yay and test/nay
 * are read with the rest of the corpus.
 */
class TaggedTest {

  private static final Path MEH = Paths.get("..", "test", "meh");

  @TestFactory
  Stream<DynamicTest> testMehCasesRefused() throws IOException {
    List<Path> paths;
    try (Stream<Path> list = Files.list(MEH)) {
      paths =
          list.filter(
                  p -> {
                    String name = p.getFileName().toString();
                    return name.startsWith("tagged-") && name.endsWith(".meh");
                  })
              .sorted()
              .toList();
    }
    assertTrue(paths.size() >= 3, "MEH tagged cases in " + MEH);
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
}
