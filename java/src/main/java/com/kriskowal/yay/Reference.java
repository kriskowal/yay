package com.kriskowal.yay;

import java.util.Objects;

/**
 * A reference, {@code <https://example.com/>}: a URI or IRI, held as written, without its angle
 * brackets. YAY never dereferences it. Two references are equal when their texts are, character
 * for character; a reference never equals a string.
 */
public record Reference(String uri) {

  public Reference {
    Objects.requireNonNull(uri, "uri");
  }

  @Override
  public String toString() {
    return "<" + uri + ">";
  }
}
