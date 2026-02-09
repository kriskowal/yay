package com.kriskowal.yay;

import java.math.BigInteger;
import java.util.*;
import java.util.regex.Pattern;

/**
 * YAY Parser - A parser for the YAY data format.
 *
 * <p>Type mapping: - null -> null - boolean -> Boolean - big integer -> BigInteger - float64 ->
 * Double - string -> String - array -> List<Object> - object -> LinkedHashMap<String, Object> -
 * bytes -> byte[]
 */
public class Yay {

  private static final Pattern INTEGER_PATTERN = Pattern.compile("^-?[0-9][0-9_]*$");
  private static final Pattern FLOAT_PATTERN = Pattern.compile("^-?[0-9_]*\\.?[0-9_]*$");

  /** Parse YAY-encoded data. */
  public static Object parse(String source) {
    return parse(source, null);
  }

  /** Parse YAY-encoded data with filename for error messages. */
  public static Object parse(String source, String filename) {
    Parser parser = new Parser(source, filename);
    return parser.parse();
  }

  // ========================================================================
  // Scanner - Phase 1: Convert source to scan lines
  // ========================================================================

  private static class ScanLine {
    String content;
    int indent;
    String leader;
    int lineNum;

    ScanLine(String content, int indent, String leader, int lineNum) {
      this.content = content;
      this.indent = indent;
      this.leader = leader;
      this.lineNum = lineNum;
    }
  }

  private static boolean isAllowedCodePoint(int cp) {
    return cp == 0x000A
        || (0x0020 <= cp && cp <= 0x007E)
        || (0x00A0 <= cp && cp <= 0xD7FF)
        || (0xE000 <= cp && cp <= 0xFFFD && !(0xFDD0 <= cp && cp <= 0xFDEF))
        || (0x10000 <= cp && cp <= 0x10FFFF && (cp & 0xFFFF) < 0xFFFE);
  }

  private static List<ScanLine> scan(String source, String filename) {
    // Check for BOM
    if (source.length() >= 1 && source.charAt(0) == '\uFEFF') {
      throw new YayException("Illegal BOM", filename, 1, 1);
    }

    // Check for forbidden code points
    {
      int line = 1, col = 1;
      for (int i = 0; i < source.length(); ) {
        int cp = source.codePointAt(i);
        if (!isAllowedCodePoint(cp)) {
          if (cp == 0x09) {
            throw new YayException("Tab not allowed (use spaces)", filename, line, col);
          }
          if (cp >= 0xD800 && cp <= 0xDFFF) {
            throw new YayException("Illegal surrogate", filename, line, col);
          }
          throw new YayException(
              String.format("Forbidden code point U+%04X", cp), filename, line, col);
        }
        if (cp == 0x0A) {
          line++;
          col = 1;
        } else {
          col++;
        }
        i += Character.charCount(cp);
      }
    }

    List<ScanLine> lines = new ArrayList<>();
    String[] rawLines = source.split("\n", -1);

    for (int i = 0; i < rawLines.length; i++) {
      String line = rawLines[i];

      // Check for trailing space
      if (line.length() > 0 && line.charAt(line.length() - 1) == ' ') {
        throw new YayException("Unexpected trailing space", filename, i + 1, line.length());
      }

      // Count indent
      int indent = 0;
      while (indent < line.length() && line.charAt(indent) == ' ') {
        indent++;
      }

      String rest = line.substring(indent);

      // Empty line or comment-only at column 0
      if (rest.isEmpty() || (rest.startsWith("#") && indent == 0)) {
        lines.add(new ScanLine("", indent, "", i));
        continue;
      }

      // Check for $ character
      if (rest.equals("$")) {
        throw new YayException("Unexpected character \"$\"", filename, i + 1, indent + 1);
      }

      // Check for * character (not allowed as leader)
      if (rest.equals("*")
          || (rest.length() >= 2 && rest.charAt(0) == '*' && rest.charAt(1) == ' ')) {
        throw new YayException("Unexpected character \"*\"", filename, i + 1, indent + 1);
      }

      // Check for list item
      String leader = "";
      String content = rest;
      if (rest.startsWith("- ")) {
        leader = "- ";
        content = rest.substring(2);
      } else if (rest.equals("-")) {
        leader = "-";
        content = "";
      } else if (rest.startsWith("-")
          && rest.length() >= 2
          && rest.charAt(1) != ' '
          && rest.charAt(1) != '.'
          && !Character.isDigit(rest.charAt(1))
          && !rest.equals("-infinity")) {
        // Compact list syntax (-value without space) is not allowed
        throw new YayException("Expected space after \"-\"", filename, i + 1, indent + 2);
      }

      lines.add(new ScanLine(content, indent, leader, i));
    }

    return lines;
  }

  // ========================================================================
  // Lexer - Phase 2: Convert scan lines to tokens
  // ========================================================================

  private enum TokenType {
    START,
    STOP,
    TEXT,
    BREAK
  }

  private static class Token {
    TokenType type;
    String text;
    int indent;
    int lineNum;
    int col;

    Token(TokenType type, String text, int indent, int lineNum, int col) {
      this.type = type;
      this.text = text;
      this.indent = indent;
      this.lineNum = lineNum;
      this.col = col;
    }
  }

  private static List<Token> tokenize(List<ScanLine> scanLines, String filename) {
    List<Token> tokens = new ArrayList<>();
    Deque<Integer> indentStack = new ArrayDeque<>();
    indentStack.push(-1);
    boolean lastWasBreak = false;

    for (ScanLine sl : scanLines) {
      // Handle blank lines
      if (sl.content.isEmpty() && sl.leader.isEmpty()) {
        if (!lastWasBreak && !tokens.isEmpty()) {
          tokens.add(new Token(TokenType.BREAK, "", sl.indent, sl.lineNum, 0));
          lastWasBreak = true;
        }
        continue;
      }

      lastWasBreak = false;
      int effectiveIndent = sl.indent + sl.leader.length();

      // Handle dedent
      while (indentStack.peek() >= effectiveIndent) {
        indentStack.pop();
        tokens.add(new Token(TokenType.STOP, "", sl.indent, sl.lineNum, 0));
      }

      // Handle list item
      if (!sl.leader.isEmpty()) {
        tokens.add(new Token(TokenType.START, sl.leader, sl.indent, sl.lineNum, sl.indent));
        indentStack.push(effectiveIndent);
      }

      // Handle content
      if (!sl.content.isEmpty()) {
        // Use indent as col (like JS), not indent + leader.length()
        tokens.add(new Token(TokenType.TEXT, sl.content, effectiveIndent, sl.lineNum, sl.indent));
      }
    }

    // Close remaining blocks
    while (indentStack.size() > 1) {
      indentStack.pop();
      tokens.add(new Token(TokenType.STOP, "", 0, scanLines.size(), 0));
    }

    return tokens;
  }

  // ========================================================================
  // Parser - Phase 3: Parse tokens into values
  // ========================================================================

  private static class Parser {
    private final List<Token> tokens;
    private final String filename;
    private int pos;

    Parser(String source, String filename) {
      this.filename = filename;

      // BOM check is now in scan()
      List<ScanLine> scanLines = scan(source, filename);
      this.tokens = tokenize(scanLines, filename);
      this.pos = 0;
    }

    Object parse() {
      skipBreaks();
      if (pos >= tokens.size()) {
        throw new YayException("No value found in document", filename);
      }

      Token t = tokens.get(pos);

      // Check for unexpected indent at root level
      if (t.type == TokenType.TEXT && t.indent > 0) {
        throw new YayException("Unexpected indent", filename, t.lineNum + 1, 1);
      }

      // Check for leading space in text
      if (t.type == TokenType.TEXT && t.text.startsWith(" ")) {
        throw new YayException("Unexpected leading space", filename, t.lineNum + 1, t.col + 1);
      }

      // Check for root-level multiline object (key: value at indent 0)
      if (t.type == TokenType.TEXT && t.indent == 0 && !t.text.startsWith("{")) {
        int colonIdx = findColonOutsideQuotes(t.text);
        if (colonIdx > 0) {
          Object result = parseRootObject();
          return ensureAtEnd(result);
        }
      }

      Object result = parseValue();
      return ensureAtEnd(result);
    }

    private Object ensureAtEnd(Object result) {
      skipBreaksAndStops();
      if (pos < tokens.size()) {
        Token t = tokens.get(pos);
        throw new YayException("Unexpected extra content", filename, t.lineNum + 1, t.col + 1);
      }
      return result;
    }

    private Map<String, Object> parseRootObject() {
      Map<String, Object> obj = new LinkedHashMap<>();

      while (pos < tokens.size()) {
        skipBreaksAndStops();
        if (pos >= tokens.size()) break;

        Token t = tokens.get(pos);

        if (t.type != TokenType.TEXT || t.indent != 0) {
          break;
        }

        int colonIdx = findColonOutsideQuotes(t.text);
        if (colonIdx <= 0) {
          break;
        }

        Map<String, Object> pair = parseKeyValuePair(t, colonIdx);
        obj.putAll(pair);
      }

      return obj;
    }

    private void skipBreaks() {
      while (pos < tokens.size() && tokens.get(pos).type == TokenType.BREAK) {
        pos++;
      }
    }

    private void skipBreaksAndStops() {
      while (pos < tokens.size()
          && (tokens.get(pos).type == TokenType.BREAK || tokens.get(pos).type == TokenType.STOP)) {
        pos++;
      }
    }

    private Object parseValue() {
      skipBreaks();
      if (pos >= tokens.size()) {
        return null;
      }

      Token t = tokens.get(pos);

      switch (t.type) {
        case START:
          if (t.text.equals("- ") || t.text.equals("-")) {
            return parseMultilineArray();
          }
          pos++;
          return parseValue();

        case TEXT:
          return parseTextValue(t);

        case STOP:
          pos++;
          return null;

        default:
          return null;
      }
    }

    private Object parseTextValue(Token t) {
      String s = t.text;

      // Check for leading space
      if (s.startsWith(" ")) {
        throw new YayException("Unexpected leading space", filename, t.lineNum + 1, t.col + 1);
      }

      // Keywords
      if (s.equals("null")) {
        pos++;
        return null;
      }
      if (s.equals("true")) {
        pos++;
        return Boolean.TRUE;
      }
      if (s.equals("false")) {
        pos++;
        return Boolean.FALSE;
      }
      if (s.equals("infinity")) {
        pos++;
        return Double.POSITIVE_INFINITY;
      }
      if (s.equals("-infinity")) {
        pos++;
        return Double.NEGATIVE_INFINITY;
      }
      if (s.equals("nan")) {
        pos++;
        return Double.NaN;
      }

      // Block string (backtick)
      if (s.equals("`") || (s.startsWith("`") && s.length() >= 2 && s.charAt(1) == ' ')) {
        String firstLine = s.length() > 2 ? s.substring(2) : "";
        return parseBlockString(firstLine, -1);
      }

      // Block bytes (>)
      if (s.startsWith(">") && !s.contains("<")) {
        return parseBlockBytes(s, -1);
      }

      // Quoted string
      if (s.startsWith("\"") && s.endsWith("\"") && s.length() >= 2) {
        pos++;
        return parseDoubleQuotedString(s, t.lineNum, t.col);
      }
      if (s.startsWith("\"") && !s.endsWith("\"")) {
        // Unterminated double-quoted string
        throw new YayException("Unterminated string", filename, t.lineNum + 1, t.col + s.length());
      }
      if (s.startsWith("'") && s.endsWith("'") && s.length() >= 2) {
        pos++;
        return parseSingleQuotedString(s);
      }
      if (s.startsWith("'") && !s.endsWith("'")) {
        // Unterminated single-quoted string
        throw new YayException("Unterminated string", filename, t.lineNum + 1, t.col + s.length());
      }

      // Inline array
      if (s.startsWith("[")) {
        pos++;
        return parseInlineArray(s, t.lineNum, t.col);
      }

      // Inline object
      if (s.startsWith("{")) {
        pos++;
        return parseInlineObject(s, t.lineNum, t.col);
      }

      // Inline bytes
      if (s.startsWith("<") && s.contains(">")) {
        pos++;
        return parseInlineBytes(s, t.lineNum, t.col);
      }

      // Unclosed angle bracket is invalid
      if (s.startsWith("<")) {
        throw new YayException("Unmatched angle bracket", filename, t.lineNum + 1, t.col + 1);
      }

      // Key:value pair
      int colonIdx = findColonOutsideQuotes(s);
      if (colonIdx > 0) {
        return parseKeyValuePair(t, colonIdx);
      }

      // Number
      Object num = parseNumber(s, t.lineNum, t.col);
      if (num != null) {
        pos++;
        return num;
      }

      // Bare words are not valid - strings must be quoted
      char firstChar = s.isEmpty() ? '?' : s.charAt(0);
      throw new YayException(
          "Unexpected character \"" + firstChar + "\"", filename, t.lineNum + 1, t.col + 1);
    }

    // ====================================================================
    // String Parsing
    // ====================================================================

    private String parseDoubleQuotedString(String s, int lineNum, int col) {
      StringBuilder out = new StringBuilder();
      int len = s.length();

      for (int i = 1; i < len - 1; i++) {
        char c = s.charAt(i);

        if (c == '\\') {
          if (i + 1 >= len - 1) {
            throw new YayException("Bad escaped character", filename, lineNum + 1, col + i + 1);
          }
          char esc = s.charAt(++i);
          switch (esc) {
            case '"':
              out.append('"');
              break;
            case '\\':
              out.append('\\');
              break;
            case '/':
              out.append('/');
              break;
            case 'b':
              out.append('\b');
              break;
            case 'f':
              out.append('\f');
              break;
            case 'n':
              out.append('\n');
              break;
            case 'r':
              out.append('\r');
              break;
            case 't':
              out.append('\t');
              break;
            case 'u':
              // Expect backslash-u{XXXXXX} format
              if (i + 1 >= len - 1 || s.charAt(i + 1) != '{') {
                throw new YayException("Bad escaped character", filename, lineNum + 1, col + i + 1);
              }
              int braceEnd = s.indexOf('}', i + 2);
              if (braceEnd < 0 || braceEnd >= len - 1) {
                throw new YayException("Bad Unicode escape", filename, lineNum + 1, col + i + 2);
              }
              String hex = s.substring(i + 2, braceEnd);
              if (hex.isEmpty() || hex.length() > 6) {
                throw new YayException("Bad Unicode escape", filename, lineNum + 1, col + i + 2);
              }
              try {
                int code = Integer.parseInt(hex, 16);
                if (code >= 0xD800 && code <= 0xDFFF) {
                  throw new YayException("Illegal surrogate", filename, lineNum + 1, col + i + 2);
                }
                if (code > 0x10FFFF) {
                  throw new YayException(
                      "Unicode code point out of range", filename, lineNum + 1, col + i + 2);
                }
                out.appendCodePoint(code);
              } catch (NumberFormatException e) {
                throw new YayException("Bad Unicode escape", filename, lineNum + 1, col + i + 2);
              }
              i = braceEnd;
              break;
            default:
              throw new YayException("Bad escaped character", filename, lineNum + 1, col + i + 1);
          }
        } else if (c < 0x20) {
          throw new YayException("Bad character in string", filename, lineNum + 1, col + i + 1);
        } else {
          out.append(c);
        }
      }

      return out.toString();
    }

    private String parseSingleQuotedString(String s) {
      StringBuilder out = new StringBuilder();
      String inner = s.substring(1, s.length() - 1);

      for (int i = 0; i < inner.length(); i++) {
        char c = inner.charAt(i);
        if (c == '\\' && i + 1 < inner.length()) {
          char next = inner.charAt(i + 1);
          if (next == '\'' || next == '\\') {
            out.append(next);
            i++;
            continue;
          }
        }
        out.append(c);
      }

      return out.toString();
    }

    private String parseBlockString(String firstLine, int baseIndent) {
      pos++;
      boolean isProperty = baseIndent >= 0;

      List<String> lines = new ArrayList<>();
      List<Integer> indents = new ArrayList<>();

      if (!firstLine.isEmpty()) {
        lines.add(firstLine);
        indents.add(-1);
      }

      while (pos < tokens.size()) {
        Token t = tokens.get(pos);
        if (t.type == TokenType.TEXT) {
          if (isProperty && t.indent <= baseIndent) {
            break;
          }
          lines.add(t.text);
          indents.add(t.indent);
          pos++;
        } else if (t.type == TokenType.BREAK) {
          lines.add("");
          indents.add(-2);
          pos++;
        } else {
          break;
        }
      }

      // Find minimum indent
      int minIndent = Integer.MAX_VALUE;
      for (int i = 0; i < indents.size(); i++) {
        if (indents.get(i) >= 0 && indents.get(i) < minIndent) {
          minIndent = indents.get(i);
        }
      }
      if (minIndent == Integer.MAX_VALUE) minIndent = 0;

      // Skip leading empty lines
      int start = 0;
      if (firstLine.isEmpty()) {
        while (start < lines.size() && lines.get(start).isEmpty()) {
          start++;
        }
      }

      // Skip trailing empty lines
      int end = lines.size();
      while (end > start && lines.get(end - 1).isEmpty()) {
        end--;
      }

      // Build result
      StringBuilder result = new StringBuilder();
      boolean leadingNewline = firstLine.isEmpty() && end > start && !isProperty;

      if (leadingNewline) {
        result.append('\n');
      }

      for (int i = start; i < end; i++) {
        if (i > start) {
          result.append('\n');
        }
        int extra = indents.get(i) >= 0 ? indents.get(i) - minIndent : 0;
        for (int j = 0; j < extra; j++) {
          result.append(' ');
        }
        result.append(lines.get(i));
      }

      if (end > start) {
        result.append('\n');
      }

      String body = result.toString();
      if (body.isEmpty()) {
        throw new YayException(
            "Empty block string not allowed (use \"\" or \"\\n\" explicitly)", filename);
      }
      return body;
    }

    // ====================================================================
    // Bytes Parsing
    // ====================================================================

    private byte[] parseInlineBytes(String s, int lineNum, int col) {
      if (s.equals("<>")) {
        return new byte[0];
      }

      int end = s.indexOf('>');

      // Check for space before >
      if (end > 1 && s.charAt(end - 1) == ' ') {
        throw new YayException("Unexpected space before \">\"", filename, lineNum + 1, col + end);
      }

      String inner = s.substring(1, end);

      // Check for uppercase hex digits
      for (int i = 0; i < inner.length(); i++) {
        char ch = inner.charAt(i);
        if (ch >= 'A' && ch <= 'F') {
          throw new YayException(
              "Uppercase hex digit (use lowercase)", filename, lineNum + 1, col + 1 + i + 1);
        }
      }

      String hex = inner.replaceAll("\\s", "");

      if (hex.length() % 2 != 0) {
        throw new YayException(
            "Odd number of hex digits in byte literal", filename, lineNum + 1, col + 1);
      }

      return hexToBytes(hex, lineNum, col);
    }

    private byte[] parseBlockBytes(String firstLine, int baseIndent) {
      Token first = tokens.get(pos);
      int firstLineNum = first.lineNum;
      int firstCol = first.col;
      pos++;
      boolean isProperty = baseIndent >= 0;

      String hexStart =
          firstLine.startsWith("> ") ? firstLine.substring(2) : firstLine.substring(1);
      int hashIdx = hexStart.indexOf('#');
      if (hashIdx >= 0) {
        hexStart = hexStart.substring(0, hashIdx);
      }

      // Check for empty leader (just ">")
      if (firstLine.equals(">") && baseIndent < 0) {
        throw new YayException("Expected hex or comment in hex block", filename);
      }

      StringBuilder hex = new StringBuilder(hexStart.replaceAll("\\s", "").toLowerCase());

      while (pos < tokens.size()) {
        Token t = tokens.get(pos);
        if (t.type == TokenType.TEXT) {
          if (isProperty && t.indent <= baseIndent) {
            break;
          }
          String line = t.text;
          int h = line.indexOf('#');
          if (h >= 0) {
            line = line.substring(0, h);
          }
          hex.append(line.replaceAll("\\s", "").toLowerCase());
          pos++;
        } else {
          break;
        }
      }

      String hexStr = hex.toString();
      if (hexStr.length() % 2 != 0) {
        throw new YayException(
            "Odd number of hex digits", filename, firstLineNum + 1, firstCol + 1);
      }

      return hexToBytes(hexStr, firstLineNum, firstCol);
    }

    private byte[] parseMultilineAngleBytes(int baseIndent) {
      Token first = tokens.get(pos);
      String firstLine = first.text;
      String hexStart =
          firstLine.startsWith("< ") ? firstLine.substring(2) : firstLine.substring(1);
      int hashIdx = hexStart.indexOf('#');
      if (hashIdx >= 0) {
        hexStart = hexStart.substring(0, hashIdx);
      }

      StringBuilder hex = new StringBuilder(hexStart.replaceAll("\\s", "").toLowerCase());
      pos++;

      while (pos < tokens.size()) {
        Token t = tokens.get(pos);
        if (t.type == TokenType.TEXT && t.indent > baseIndent) {
          String line = t.text;
          int h = line.indexOf('#');
          if (h >= 0) {
            line = line.substring(0, h);
          }
          hex.append(line.replaceAll("\\s", "").toLowerCase());
          pos++;
        } else {
          break;
        }
      }

      String hexStr = hex.toString();
      if (hexStr.length() % 2 != 0) {
        throw new YayException(
            "Odd number of hex digits", filename, first.lineNum + 1, first.col + 1);
      }

      return hexToBytes(hexStr, first.lineNum, first.col);
    }

    private byte[] hexToBytes(String hex, int lineNum, int col) {
      // Validate hex digits
      for (int i = 0; i < hex.length(); i++) {
        char ch = hex.charAt(i);
        boolean isHex =
            (ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F');
        if (!isHex) {
          throw new YayException("Invalid hex digit", filename, lineNum + 1, col + 1);
        }
      }
      byte[] bytes = new byte[hex.length() / 2];
      for (int i = 0; i < bytes.length; i++) {
        bytes[i] = (byte) Integer.parseInt(hex.substring(i * 2, i * 2 + 2), 16);
      }
      return bytes;
    }

    // ====================================================================
    // Array Parsing
    // ====================================================================

    private List<Object> parseMultilineArray() {
      return parseMultilineArrayImpl(-1);
    }

    private List<Object> parseMultilineArrayImpl(int minIndent) {
      List<Object> items = new ArrayList<>();
      int baseIndent = tokens.get(pos).indent;

      while (pos < tokens.size()) {
        Token t = tokens.get(pos);

        // Stop if we encounter a list item at a lower indent than expected
        if (minIndent >= 0 && t.type == TokenType.START && t.indent < minIndent) {
          break;
        }

        if (t.type == TokenType.START && t.indent == baseIndent) {
          int listItemIndent = t.indent;
          pos++;
          skipBreaks();
          if (pos < tokens.size()) {
            Token next = tokens.get(pos);
            // Check for nested array (text starting with "- ")
            if (next.type == TokenType.TEXT && next.text.startsWith("- ")) {
              items.add(parseInlineBulletList(baseIndent));
            } else if (next.type == TokenType.START) {
              // Another START token means nested list
              items.add(parseMultilineArray());
            } else if (next.type == TokenType.TEXT && findColonOutsideQuotes(next.text) > 0) {
              // Object inside array item - parse all key-value pairs at this indent level
              items.add(parseObjectInArrayItem(listItemIndent));
            } else {
              items.add(parseValue());
            }
          } else {
            items.add(null);
          }
          skipBreaksAndStops();
        } else if (t.type == TokenType.STOP) {
          pos++;
        } else if (t.type == TokenType.BREAK) {
          pos++;
        } else if (t.indent <= baseIndent) {
          break;
        } else {
          break;
        }
      }

      return items;
    }

    // Parse an object that appears as an array item
    private Map<String, Object> parseObjectInArrayItem(int listItemIndent) {
      Map<String, Object> obj = new LinkedHashMap<>();
      int objectIndent = -1;

      while (pos < tokens.size()) {
        skipBreaksAndStops();
        if (pos >= tokens.size()) break;

        Token t = tokens.get(pos);

        // First key-value pair determines the object indent
        if (objectIndent < 0 && t.type == TokenType.TEXT) {
          objectIndent = t.indent;
        }

        // Stop if we've dedented past the object indent
        if (t.type == TokenType.TEXT && t.indent < objectIndent) {
          break;
        }

        // Stop if we hit a new list item at the same or lower indent
        if (t.type == TokenType.START && t.indent <= listItemIndent) {
          break;
        }

        // Parse key-value pairs at the object indent
        if (t.type == TokenType.TEXT && t.indent == objectIndent) {
          int colonIdx = findColonOutsideQuotes(t.text);
          if (colonIdx > 0) {
            Map<String, Object> pair = parseKeyValuePair(t, colonIdx);
            obj.putAll(pair);
          } else {
            pos++;
          }
        } else {
          pos++;
        }
      }

      return obj;
    }

    // Recursively parse nested inline bullets like "- - - value"
    private Object parseNestedInlineBullet(String text, int lineNum, int col) {
      if (text.startsWith("- ")) {
        String innerText = text.substring(2);
        // Check for extra space after "-"
        if (innerText.startsWith(" ")) {
          throw new YayException("Unexpected space after \"-\"", filename, lineNum + 1, col + 3);
        }
        Object innerVal = parseNestedInlineBullet(innerText, lineNum, col + 2);
        List<Object> arr = new ArrayList<>();
        arr.add(innerVal);
        return arr;
      }
      return parseScalar(text, lineNum, col);
    }

    // Parse inline bullet list like "- a" followed by "  - b"
    private List<Object> parseInlineBulletList(int listStartIndent) {
      List<Object> items = new ArrayList<>();

      while (pos < tokens.size()) {
        Token t = tokens.get(pos);

        // Handle inline bullet text (e.g., "- a" or "- key:")
        if (t.type == TokenType.TEXT && t.text.startsWith("- ")) {
          String value = t.text.substring(2);

          // Check for extra space after "-"
          if (value.startsWith(" ")) {
            throw new YayException(
                "Unexpected space after \"-\"", filename, t.lineNum + 1, t.col + 3);
          }

          // Check for key-value pair, but only if not starting with { or [
          // (those are inline objects/arrays that may contain colons)
          int colonIdx = -1;
          if (!value.startsWith("{") && !value.startsWith("[")) {
            colonIdx = findColonOutsideQuotes(value);
          }

          if (colonIdx > 0) {
            // This is a key-value pair like "- e:" - parse as object
            pos++;
            String key = value.substring(0, colonIdx).trim();
            String valuePart =
                colonIdx + 1 < value.length() ? value.substring(colonIdx + 1).trim() : "";

            Map<String, Object> obj = new LinkedHashMap<>();
            if (!valuePart.isEmpty()) {
              obj.put(key, parseScalar(valuePart, t.lineNum, t.col + 2 + colonIdx + 2));
            } else {
              // Nested content - parse object at deeper indent
              skipBreaksAndStops();
              if (pos < tokens.size()) {
                Token next = tokens.get(pos);
                if (next.type == TokenType.TEXT && next.indent > t.indent) {
                  obj.put(key, parseNestedObject(t.indent));
                } else {
                  obj.put(key, null);
                }
              } else {
                obj.put(key, null);
              }
            }
            items.add(obj);
          } else {
            pos++;
            // Use parseNestedInlineBullet to handle "- - - value" and inline objects/arrays
            items.add(parseNestedInlineBullet(value, t.lineNum, t.col + 2));
          }
          continue;
        }

        // Handle continuation START tokens (e.g., the "- " in "  - b")
        if (t.type == TokenType.START && t.indent > listStartIndent) {
          int itemIndent = t.indent;
          pos++;
          skipBreaks();
          if (pos < tokens.size()) {
            Token next = tokens.get(pos);
            if (next.type == TokenType.TEXT) {
              int colonIdx = findColonOutsideQuotes(next.text);
              if (colonIdx > 0) {
                // Object inside list item
                items.add(parseObjectInArrayItem(itemIndent));
              } else {
                items.add(parseScalar(next.text, next.lineNum, next.col));
                pos++;
              }
            } else {
              items.add(parseValue());
            }
          }
          skipBreaksAndStops();
          continue;
        }

        // Handle STOP tokens
        if (t.type == TokenType.STOP) {
          pos++;
          continue;
        }

        // Handle BREAK tokens
        if (t.type == TokenType.BREAK) {
          pos++;
          continue;
        }

        // Exit on anything else
        break;
      }

      return items;
    }

    private List<Object> parseInlineArray(String s, int lineNum, int col) {
      // Check for multiline array (no closing bracket)
      if (!s.contains("]")) {
        throw new YayException(
            "Unexpected newline in inline array", filename, lineNum + 1, col + 1);
      }

      // Validate whitespace in inline array
      validateInlineArrayWhitespace(s, lineNum, col);

      List<Object> items = new ArrayList<>();

      if (s.equals("[]")) {
        return items;
      }

      int i = 1;
      while (i < s.length() - 1) {
        // Skip single space after comma (already validated)
        if (s.charAt(i) == ' ') {
          i++;
        }

        if (i >= s.length() - 1) break;

        int[] consumed = new int[1];
        Object value = parseInlineValue(s, i, consumed, lineNum, col);
        items.add(value);
        i = consumed[0];

        // Skip comma
        if (i < s.length() - 1 && s.charAt(i) == ',') {
          i++;
        }
      }

      return items;
    }

    private void validateInlineArrayWhitespace(String s, int lineNum, int col) {
      boolean inSingle = false;
      boolean inDouble = false;
      boolean escape = false;
      int depth = 0;

      for (int i = 0; i < s.length(); i++) {
        char ch = s.charAt(i);

        if (escape) {
          escape = false;
          continue;
        }

        if (inSingle) {
          if (ch == '\\') escape = true;
          else if (ch == '\'') inSingle = false;
          continue;
        }

        if (inDouble) {
          if (ch == '\\') escape = true;
          else if (ch == '"') inDouble = false;
          continue;
        }

        if (ch == '\'') {
          inSingle = true;
          continue;
        }
        if (ch == '"') {
          inDouble = true;
          continue;
        }

        if (ch == '[') {
          depth++;
          if (i + 1 < s.length() && s.charAt(i + 1) == ' ') {
            throw new YayException(
                "Unexpected space after \"[\"", filename, lineNum + 1, col + i + 2);
          }
          continue;
        }

        if (ch == ']') {
          if (i > 0 && s.charAt(i - 1) == ' ') {
            throw new YayException("Unexpected space before \"]\"", filename, lineNum + 1, col + i);
          }
          if (depth > 0) depth--;
          continue;
        }

        if (ch == ',') {
          if (i > 0 && s.charAt(i - 1) == ' ') {
            throw new YayException("Unexpected space before \",\"", filename, lineNum + 1, col + i);
          }
          if (i + 1 < s.length() && s.charAt(i + 1) != ' ' && s.charAt(i + 1) != ']') {
            // Lookahead to check if closing ] has space before it
            boolean nextIsClosingWithSpace = false;
            int lookaheadDepth = depth;
            boolean inS = false, inD = false, esc = false;
            for (int j = i + 1; j < s.length(); j++) {
              char cj = s.charAt(j);
              if (esc) {
                esc = false;
                continue;
              }
              if (inS) {
                if (cj == '\\') esc = true;
                else if (cj == '\'') inS = false;
                continue;
              }
              if (inD) {
                if (cj == '\\') esc = true;
                else if (cj == '"') inD = false;
                continue;
              }
              if (cj == '\'') {
                inS = true;
                continue;
              }
              if (cj == '"') {
                inD = true;
                continue;
              }
              if (cj == '[') {
                lookaheadDepth++;
                continue;
              }
              if (cj == ']') {
                if (lookaheadDepth == depth) {
                  nextIsClosingWithSpace = j > 0 && s.charAt(j - 1) == ' ';
                  break;
                }
                if (lookaheadDepth > 0) lookaheadDepth--;
                continue;
              }
              if (cj == ',' && lookaheadDepth == depth) {
                break;
              }
            }
            if (!nextIsClosingWithSpace) {
              throw new YayException(
                  "Expected space after \",\"", filename, lineNum + 1, col + i + 1);
            }
          }
          if (i + 2 < s.length() && s.charAt(i + 1) == ' ' && s.charAt(i + 2) == ' ') {
            throw new YayException(
                "Unexpected space after \",\"", filename, lineNum + 1, col + i + 3);
          }
        }

        if (ch == '\t') {
          throw new YayException(
              "Tab not allowed (use spaces)", filename, lineNum + 1, col + i + 1);
        }
      }
    }

    // ====================================================================
    // Object Parsing
    // ====================================================================

    private Map<String, Object> parseKeyValuePair(Token t, int colonIdx) {
      String keyRaw = t.text.substring(0, colonIdx);
      String valueSlice = t.text.substring(colonIdx + 1);

      // Check for space before colon
      if (keyRaw.endsWith(" ")) {
        throw new YayException(
            "Unexpected space before \":\"", filename, t.lineNum + 1, t.col + colonIdx);
      }

      // Check for space after colon
      if (!valueSlice.isEmpty() && !valueSlice.startsWith(" ")) {
        throw new YayException(
            "Expected space after \":\"", filename, t.lineNum + 1, t.col + colonIdx + 1);
      }
      if (valueSlice.startsWith("  ")) {
        throw new YayException(
            "Unexpected space after \":\"", filename, t.lineNum + 1, t.col + colonIdx + 3);
      }

      String key = keyRaw.trim();
      String valuePart = valueSlice.trim();

      // Handle quoted keys
      if (key.startsWith("\"") && key.endsWith("\"")) {
        key = parseDoubleQuotedString(key, t.lineNum, t.col);
      } else if (key.startsWith("'") && key.endsWith("'")) {
        key = parseSingleQuotedString(key);
      } else {
        // Validate unquoted key characters (alphanumeric, underscore, hyphen)
        for (int i = 0; i < key.length(); i++) {
          char c = key.charAt(i);
          boolean isAlpha = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
          boolean isDigit = c >= '0' && c <= '9';
          boolean isUnderscore = c == '_';
          boolean isHyphen = c == '-';
          if (!isAlpha && !isDigit && !isUnderscore && !isHyphen) {
            throw new YayException("Invalid key character", filename, t.lineNum + 1, t.col + i + 1);
          }
        }
      }

      Map<String, Object> obj = new LinkedHashMap<>();

      // Empty object
      if (valuePart.equals("{}")) {
        pos++;
        obj.put(key, new LinkedHashMap<>());
        return obj;
      }

      // Block string
      if (valuePart.equals("`")
          || (valuePart.startsWith("`") && valuePart.length() >= 2 && valuePart.charAt(1) == ' ')) {
        String firstLine = valuePart.length() > 2 ? valuePart.substring(2) : "";
        // In property context, content must be on next line
        if (!firstLine.isEmpty()) {
          throw new YayException(
              "Expected newline after block leader in property",
              filename,
              t.lineNum + 1,
              t.col + colonIdx + 3);
        }
        obj.put(key, parseBlockString(firstLine, t.indent));
        return obj;
      }

      // Block bytes
      if (valuePart.startsWith(">") && !valuePart.contains("<")) {
        // In property context, content must be on next line (but comments are allowed)
        if (valuePart.length() > 1 && valuePart.charAt(1) == ' ') {
          // Check if it's a comment (starts with "> #")
          if (!(valuePart.length() >= 3 && valuePart.charAt(2) == '#')) {
            throw new YayException(
                "Expected newline after block leader in property",
                filename,
                t.lineNum + 1,
                t.col + colonIdx + 3);
          }
        }
        obj.put(key, parseBlockBytes(valuePart, t.indent));
        return obj;
      }

      // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the
      // same line

      // Inline value
      if (!valuePart.isEmpty()) {
        pos++;
        obj.put(key, parseScalar(valuePart, t.lineNum, t.col + colonIdx + 2));
        return obj;
      }

      // Nested content
      pos++;
      skipBreaksAndStops();

      if (pos >= tokens.size()) {
        // Empty property with no nested content is invalid
        throw new YayException(
            "Expected value after property", filename, t.lineNum + 1, t.col + colonIdx + 2);
      }

      Token next = tokens.get(pos);

      // Named array - pass next.indent so array stops at items below this level
      if (next.type == TokenType.START && (next.text.equals("- ") || next.text.equals("-"))) {
        obj.put(key, parseMultilineArrayImpl(next.indent));
        return obj;
      }

      // Block string on next line - this is invalid in strict YAY
      // The backtick must be on the same line as the key
      if (next.type == TokenType.TEXT && next.text.equals("`")) {
        throw new YayException("Unexpected indent", filename, next.lineNum + 1, 1);
      }

      // Block bytes on next line - this is invalid in strict YAY
      // The > must be on the same line as the key
      if (next.type == TokenType.TEXT && next.text.startsWith(">") && !next.text.contains("<")) {
        throw new YayException("Unexpected indent", filename, next.lineNum + 1, 1);
      }

      // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the
      // same line

      // Concatenated quoted strings (multiple quoted strings on consecutive lines)
      if (next.type == TokenType.TEXT && next.indent > t.indent) {
        String trimmed = next.text.trim();
        if ((trimmed.startsWith("\"") && trimmed.endsWith("\"") && trimmed.length() >= 2)
            || (trimmed.startsWith("'") && trimmed.endsWith("'") && trimmed.length() >= 2)) {
          String result = parseConcatenatedStrings(next.indent);
          if (result != null) {
            obj.put(key, result);
            return obj;
          }
          // Single string on new line is invalid - fall through to error
          throw new YayException("Unexpected indent", filename, next.lineNum + 1, 1);
        }
      }

      // Nested object
      if (next.type == TokenType.TEXT && next.indent > t.indent) {
        obj.put(key, parseNestedObject(t.indent));
        return obj;
      }

      // Empty property with no nested content is invalid
      throw new YayException(
          "Expected value after property", filename, t.lineNum + 1, t.col + colonIdx + 2);
    }

    private Map<String, Object> parseNestedObject(int baseIndent) {
      Map<String, Object> obj = new LinkedHashMap<>();

      while (pos < tokens.size()) {
        skipBreaksAndStops();
        if (pos >= tokens.size()) break;

        Token t = tokens.get(pos);

        if (t.type != TokenType.TEXT || t.indent <= baseIndent) {
          break;
        }

        // Reject inline values on separate line (they look like keys starting with special chars)
        if (t.text.startsWith("{") || t.text.startsWith("[") || t.text.startsWith("<")) {
          throw new YayException("Unexpected indent", filename, t.lineNum + 1, 1);
        }

        int colonIdx = findColonOutsideQuotes(t.text);
        if (colonIdx > 0) {
          Map<String, Object> pair = parseKeyValuePair(t, colonIdx);
          obj.putAll(pair);
        } else {
          // Text without colon in nested object context is invalid
          throw new YayException("Unexpected indent", filename, t.lineNum + 1, 1);
        }
      }

      return obj;
    }

    /**
     * Parse concatenated quoted strings (multiple quoted strings on consecutive lines). Returns
     * null if there's only one string (single string on new line is invalid).
     */
    private String parseConcatenatedStrings(int baseIndent) {
      List<String> parts = new ArrayList<>();

      while (pos < tokens.size()) {
        Token t = tokens.get(pos);

        if (t.type == TokenType.BREAK || t.type == TokenType.STOP) {
          pos++;
          continue;
        }

        if (t.type != TokenType.TEXT || t.indent < baseIndent) {
          break;
        }

        String trimmed = t.text.trim();

        // Check if this line is a quoted string
        boolean isDoubleQuoted =
            trimmed.startsWith("\"") && trimmed.endsWith("\"") && trimmed.length() >= 2;
        boolean isSingleQuoted =
            trimmed.startsWith("'") && trimmed.endsWith("'") && trimmed.length() >= 2;

        if (!isDoubleQuoted && !isSingleQuoted) {
          break;
        }

        // Parse the quoted string
        String parsed;
        if (isDoubleQuoted) {
          parsed = parseDoubleQuotedString(trimmed, t.lineNum, t.col);
        } else {
          parsed = parseSingleQuotedString(trimmed);
        }
        parts.add(parsed);
        pos++;
      }

      // Require at least 2 strings for concatenation
      // A single string on a new line is invalid (use inline syntax instead)
      if (parts.size() < 2) {
        return null;
      }

      return String.join("", parts);
    }

    private byte[] parseMultilineAngleBytesProperty(int baseIndent) {
      skipBreaks();

      Token first = pos < tokens.size() ? tokens.get(pos) : null;
      int firstLineNum = first != null ? first.lineNum : 0;
      int firstCol = first != null ? first.col : 0;

      StringBuilder hex = new StringBuilder();

      while (pos < tokens.size()) {
        Token t = tokens.get(pos);
        if (t.type == TokenType.TEXT && t.indent > baseIndent) {
          String line = t.text;
          int h = line.indexOf('#');
          if (h >= 0) {
            line = line.substring(0, h);
          }
          hex.append(line.replaceAll("\\s", "").toLowerCase());
          pos++;
        } else {
          break;
        }
      }

      String hexStr = hex.toString();
      if (hexStr.isEmpty()) {
        return new byte[0];
      }
      if (hexStr.length() % 2 != 0) {
        throw new YayException(
            "Odd number of hex digits", filename, firstLineNum + 1, firstCol + 1);
      }

      return hexToBytes(hexStr, firstLineNum, firstCol);
    }

    private Map<String, Object> parseInlineObject(String s, int lineNum, int col) {
      // Check for multiline object (no closing brace)
      if (!s.contains("}")) {
        throw new YayException(
            "Unexpected newline in inline object", filename, lineNum + 1, col + 1);
      }

      // Validate whitespace in inline object
      validateInlineObjectWhitespace(s, lineNum, col);

      Map<String, Object> obj = new LinkedHashMap<>();

      if (s.equals("{}")) {
        return obj;
      }

      int i = 1;
      while (i < s.length() - 1) {
        // Skip single space after comma (already validated)
        if (s.charAt(i) == ' ') {
          i++;
        }

        if (i >= s.length() - 1) break;

        // Parse key
        int keyStart = i;
        String key;

        if (s.charAt(i) == '"' || s.charAt(i) == '\'') {
          char quote = s.charAt(i);
          i++;
          while (i < s.length() && s.charAt(i) != quote) {
            if (s.charAt(i) == '\\') i++;
            i++;
          }
          i++; // Skip closing quote
          String quotedKey = s.substring(keyStart, i);
          if (quote == '"') {
            key = parseDoubleQuotedString(quotedKey, lineNum, col + keyStart);
          } else {
            key = parseSingleQuotedString(quotedKey);
          }
        } else {
          // Parse bare key - must be alphanumeric, underscore, or hyphen
          while (i < s.length()) {
            char ch = s.charAt(i);
            boolean isAlpha = (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z');
            boolean isDigit = ch >= '0' && ch <= '9';
            boolean isUnderscore = ch == '_';
            boolean isHyphen = ch == '-';
            if (!isAlpha && !isDigit && !isUnderscore && !isHyphen) {
              // First character invalid = "Invalid key"
              if (i == keyStart) {
                throw new YayException("Invalid key", filename, lineNum + 1, col + 1);
              }
              // Stop at non-key character (colon, space, etc.)
              break;
            }
            i++;
          }
          if (i == keyStart) {
            throw new YayException("Invalid key", filename, lineNum + 1, col + 1);
          }
          key = s.substring(keyStart, i);
        }

        // Check for colon (must be immediately after key, no space allowed before)
        if (i >= s.length() || s.charAt(i) != ':') {
          throw new YayException("Expected colon after key", filename, lineNum + 1, col + 1);
        }
        i++; // Skip colon

        // Skip space after colon
        if (i < s.length() && s.charAt(i) == ' ') {
          i++;
        }

        // Parse value
        int[] consumed = new int[1];
        Object value = parseInlineValue(s, i, consumed, lineNum, col);
        obj.put(key, value);
        i = consumed[0];

        // Skip comma
        if (i < s.length() - 1 && s.charAt(i) == ',') {
          i++;
        }
      }

      return obj;
    }

    private void validateInlineObjectWhitespace(String s, int lineNum, int col) {
      boolean inSingle = false;
      boolean inDouble = false;
      boolean escape = false;

      for (int i = 0; i < s.length(); i++) {
        char ch = s.charAt(i);

        if (escape) {
          escape = false;
          continue;
        }

        if (inSingle) {
          if (ch == '\\') escape = true;
          else if (ch == '\'') inSingle = false;
          continue;
        }

        if (inDouble) {
          if (ch == '\\') escape = true;
          else if (ch == '"') inDouble = false;
          continue;
        }

        if (ch == '\'') {
          inSingle = true;
          continue;
        }
        if (ch == '"') {
          inDouble = true;
          continue;
        }

        if (ch == '{') {
          if (i + 1 < s.length() && s.charAt(i + 1) == ' ') {
            throw new YayException(
                "Unexpected space after \"{\"", filename, lineNum + 1, col + i + 2);
          }
          continue;
        }

        if (ch == '}') {
          if (i > 0 && s.charAt(i - 1) == ' ') {
            throw new YayException("Unexpected space before \"}\"", filename, lineNum + 1, col + i);
          }
          continue;
        }

        if (ch == ',') {
          if (i > 0 && s.charAt(i - 1) == ' ') {
            throw new YayException("Unexpected space before \",\"", filename, lineNum + 1, col + i);
          }
          if (i + 1 < s.length() && s.charAt(i + 1) != ' ' && s.charAt(i + 1) != '}') {
            throw new YayException(
                "Expected space after \",\"", filename, lineNum + 1, col + i + 1);
          }
          if (i + 2 < s.length() && s.charAt(i + 1) == ' ' && s.charAt(i + 2) == ' ') {
            throw new YayException(
                "Unexpected space after \",\"", filename, lineNum + 1, col + i + 3);
          }
        }

        if (ch == '\t') {
          throw new YayException(
              "Tab not allowed (use spaces)", filename, lineNum + 1, col + i + 1);
        }
      }
    }

    // ====================================================================
    // Inline Value Parsing
    // ====================================================================

    private Object parseInlineValue(String s, int start, int[] consumed, int lineNum, int col) {
      if (start >= s.length()) {
        consumed[0] = start;
        return null;
      }

      char c = s.charAt(start);

      // Quoted string
      if (c == '"' || c == '\'') {
        int end = findMatchingQuote(s, start);
        String quoted = s.substring(start, end + 1);
        consumed[0] = end + 1;
        if (c == '"') {
          return parseDoubleQuotedString(quoted, lineNum, col + start);
        } else {
          return parseSingleQuotedString(quoted);
        }
      }

      // Nested array
      if (c == '[') {
        int end = findMatchingBracket(s, start, '[', ']');
        String inner = s.substring(start, end + 1);
        consumed[0] = end + 1;
        return parseInlineArray(inner, lineNum, col + start);
      }

      // Nested object
      if (c == '{') {
        int end = findMatchingBracket(s, start, '{', '}');
        String inner = s.substring(start, end + 1);
        consumed[0] = end + 1;
        return parseInlineObject(inner, lineNum, col + start);
      }

      // Bytes
      if (c == '<') {
        int end = s.indexOf('>', start);
        if (end < 0) end = s.length() - 1;
        String inner = s.substring(start, end + 1);
        consumed[0] = end + 1;
        return parseInlineBytes(inner, lineNum, col + start);
      }

      // Find end of value (comma or closing bracket)
      int end = start;
      while (end < s.length()
          && s.charAt(end) != ','
          && s.charAt(end) != ']'
          && s.charAt(end) != '}') {
        end++;
      }

      String valueStr = s.substring(start, end).trim();
      consumed[0] = end;

      return parseScalar(valueStr, lineNum, col + start);
    }

    private String stripInlineComment(String s) {
      boolean inDouble = false;
      boolean inSingle = false;
      boolean escape = false;

      for (int i = 0; i < s.length(); i++) {
        char c = s.charAt(i);
        if (escape) {
          escape = false;
          continue;
        }
        if (c == '\\') {
          escape = true;
          continue;
        }
        if (c == '"' && !inSingle) {
          inDouble = !inDouble;
        } else if (c == '\'' && !inDouble) {
          inSingle = !inSingle;
        } else if (c == '#' && !inDouble && !inSingle) {
          return s.substring(0, i).stripTrailing();
        }
      }
      return s;
    }

    private Object parseScalar(String s, int lineNum, int col) {
      // Strip inline comments first
      s = stripInlineComment(s);

      if (s.equals("null")) return null;
      if (s.equals("true")) return Boolean.TRUE;
      if (s.equals("false")) return Boolean.FALSE;
      if (s.equals("infinity")) return Double.POSITIVE_INFINITY;
      if (s.equals("-infinity")) return Double.NEGATIVE_INFINITY;
      if (s.equals("nan")) return Double.NaN;

      // Quoted string
      if (s.startsWith("\"") && s.endsWith("\"")) {
        return parseDoubleQuotedString(s, lineNum, col);
      }
      if (s.startsWith("'") && s.endsWith("'")) {
        return parseSingleQuotedString(s);
      }

      // Bytes
      if (s.startsWith("<")) {
        if (!s.endsWith(">")) {
          throw new YayException("Unmatched angle bracket", filename, lineNum + 1, col + 1);
        }
        return parseInlineBytes(s, lineNum, col);
      }

      // Inline array
      if (s.startsWith("[") && s.endsWith("]")) {
        return parseInlineArray(s, lineNum, col);
      }

      // Inline object
      if (s.startsWith("{") && s.endsWith("}")) {
        return parseInlineObject(s, lineNum, col);
      }

      // Number
      Object num = parseNumber(s, lineNum, col);
      if (num != null) return num;

      // Bare words are not valid - strings must be quoted
      char firstChar = s.isEmpty() ? '?' : s.charAt(0);
      throw new YayException(
          "Unexpected character \"" + firstChar + "\"", filename, lineNum + 1, col + 1);
    }

    // ====================================================================
    // Number Parsing
    // ====================================================================

    private Object parseNumber(String s) {
      return parseNumber(s, -1, -1);
    }

    private Object parseNumber(String s, int lineNum, int col) {
      // Check for uppercase E in exponent (must be lowercase)
      if (lineNum >= 0) {
        int eIdx = s.indexOf('E');
        if (eIdx >= 0) {
          throw new YayException(
              "Uppercase exponent (use lowercase 'e')", filename, lineNum + 1, col + eIdx + 1);
        }
      }

      // Check if string looks like a number (digits, spaces, dots, minus, exponent)
      boolean hasDigit = false;
      boolean hasExponent = false;
      for (int i = 0; i < s.length(); i++) {
        char c = s.charAt(i);
        if (c == ' ') continue;
        if (c >= '0' && c <= '9') {
          hasDigit = true;
          continue;
        }
        if (c == '.') continue;
        if (c == '-' && i == 0) continue;
        if (c == '_') continue;
        // Allow 'e' for exponent notation (E already rejected above)
        if (c == 'e' && hasDigit && !hasExponent) {
          hasExponent = true;
          continue;
        }
        // Allow +/- after exponent
        if ((c == '+' || c == '-') && hasExponent) {
          char prev = i > 0 ? s.charAt(i - 1) : ' ';
          if (prev == 'e') continue;
        }
        // Not a numeric candidate
        return null;
      }
      if (!hasDigit) return null;

      // Validate spaces in numbers (must be between digits)
      if (lineNum >= 0) {
        for (int i = 0; i < s.length(); i++) {
          if (s.charAt(i) != ' ') continue;
          char prev = i > 0 ? s.charAt(i - 1) : ' ';
          char next = i + 1 < s.length() ? s.charAt(i + 1) : ' ';
          boolean isPrevDigit = prev >= '0' && prev <= '9';
          boolean isNextDigit = next >= '0' && next <= '9';
          if (!(isPrevDigit && isNextDigit)) {
            throw new YayException(
                "Unexpected space in number", filename, lineNum + 1, col + i + 1);
          }
        }
      }

      // Remove underscores and spaces
      String normalized = s.replace("_", "").replace(" ", "");

      // Integer (no dot and no exponent)
      if (!normalized.contains(".") && !normalized.toLowerCase().contains("e")) {
        try {
          return new BigInteger(normalized);
        } catch (NumberFormatException e) {
          return null;
        }
      }

      // Float
      try {
        return Double.parseDouble(normalized);
      } catch (NumberFormatException e) {
        return null;
      }
    }

    // ====================================================================
    // Utility Methods
    // ====================================================================

    private int findColonOutsideQuotes(String s) {
      boolean inDouble = false;
      boolean inSingle = false;
      boolean escape = false;

      for (int i = 0; i < s.length(); i++) {
        char c = s.charAt(i);

        if (escape) {
          escape = false;
          continue;
        }

        if (c == '\\' && (inDouble || inSingle)) {
          escape = true;
          continue;
        }

        if (c == '"' && !inSingle) {
          inDouble = !inDouble;
        } else if (c == '\'' && !inDouble) {
          inSingle = !inSingle;
        } else if (c == ':' && !inDouble && !inSingle) {
          return i;
        }
      }

      return -1;
    }

    private int findMatchingQuote(String s, int start) {
      char quote = s.charAt(start);
      boolean escape = false;

      for (int i = start + 1; i < s.length(); i++) {
        char c = s.charAt(i);

        if (escape) {
          escape = false;
          continue;
        }

        if (c == '\\') {
          escape = true;
          continue;
        }

        if (c == quote) {
          return i;
        }
      }

      return s.length() - 1;
    }

    private int findMatchingBracket(String s, int start, char open, char close) {
      int depth = 0;
      boolean inString = false;
      char stringChar = 0;
      boolean escape = false;

      for (int i = start; i < s.length(); i++) {
        char c = s.charAt(i);

        if (escape) {
          escape = false;
          continue;
        }

        if (c == '\\' && inString) {
          escape = true;
          continue;
        }

        if ((c == '"' || c == '\'') && (!inString || c == stringChar)) {
          if (inString) {
            inString = false;
          } else {
            inString = true;
            stringChar = c;
          }
          continue;
        }

        if (!inString) {
          if (c == open) {
            depth++;
          } else if (c == close) {
            depth--;
            if (depth == 0) {
              return i;
            }
          }
        }
      }

      return s.length() - 1;
    }
  }

  // ========================================================================
  // Exception
  // ========================================================================

  public static class YayException extends RuntimeException {
    public YayException(String message, String filename, int line, int col) {
      super(
          message + " at " + line + ":" + col + (filename != null ? " of <" + filename + ">" : ""));
    }

    public YayException(String message, String filename) {
      super(message + (filename != null ? " <" + filename + ">" : ""));
    }
  }
}
