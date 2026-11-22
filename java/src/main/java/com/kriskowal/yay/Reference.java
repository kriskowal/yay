package com.kriskowal.yay;

import java.util.Objects;

/**
 * A reference, {@code <https://example.com/>}: a URI or IRI, held as written, without its angle
 * brackets. YAY never dereferences it. Two references are equal when their texts are, character
 * for character; a reference never equals a string.
 *
 * <p>The constructor refuses, with an {@link IllegalArgumentException}, text that is not a
 * reference's by the parser's own rule: a lowercase scheme, a colon, URI or IRI characters and
 * well-formed percent-encodings, with nothing after.
 */
public record Reference(String uri) {

  public Reference {
    Objects.requireNonNull(uri, "uri");
    String why = Yay.referenceFault(uri);
    if (why != null) {
      throw new IllegalArgumentException("Not a reference: \"" + uri + "\" (" + why + ")");
    }
  }

  @Override
  public String toString() {
    return "<" + uri + ">";
  }
}
