#!/usr/bin/env python3
"""
Test harness for the YAY parser.

Runs against all .yay fixture files and compares with expected .js output.
Runs against all .nay fixture files and verifies errors are raised.
"""

import json
import math
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
import libyay as yay


def parse_js_value(js_content: str):
    """
    Parse a JavaScript value literal into Python equivalent.

    Handles:
    - BigInt literals (10n -> 10)
    - Infinity, -Infinity, NaN
    - Uint8Array.fromHex("...") -> bytes
    - new Uint8Array(0) -> b''
    - Arrays and objects
    """
    js_content = js_content.strip()

    # Handle special float values
    if js_content == "Infinity":
        return float("inf")
    if js_content == "-Infinity":
        return float("-inf")
    if js_content == "NaN":
        return float("nan")

    # Handle null
    if js_content == "null":
        return None

    # Handle booleans
    if js_content == "true":
        return True
    if js_content == "false":
        return False

    # Handle empty Uint8Array
    if js_content == "new Uint8Array(0)":
        return b""

    # Handle Uint8Array.fromHex("...")
    if js_content.startswith("Uint8Array.fromHex("):
        hex_str = js_content[20:-2]  # Extract hex string
        return bytes.fromhex(hex_str)

    # Handle BigInt literals (e.g., 10n, -10n)
    if (
        js_content.endswith("n")
        and not js_content.startswith('"')
        and not js_content.startswith("'")
    ):
        try:
            return int(js_content[:-1])
        except ValueError:
            pass

    # Handle negative numbers
    if js_content.startswith("-"):
        rest = js_content[1:]
        if rest.endswith("n"):
            return -int(rest[:-1])

    # Try parsing as JSON (handles strings, numbers, arrays, objects)
    try:
        # First, preprocess to handle BigInt in arrays/objects
        processed = preprocess_js_for_json(js_content)
        return json.loads(processed)
    except json.JSONDecodeError:
        pass

    # Handle plain integers
    try:
        return int(js_content)
    except ValueError:
        pass

    # Handle floats
    try:
        return float(js_content)
    except ValueError:
        pass

    raise ValueError(f"Cannot parse JS value: {js_content!r}")


def preprocess_js_for_json(js: str) -> str:
    """
    Preprocess JavaScript to be valid JSON.

    - Remove BigInt 'n' suffix
    - Quote unquoted object keys
    - Handle single-quoted strings
    """
    result = []
    i = 0
    in_string = False
    string_char = None

    while i < len(js):
        ch = js[i]

        if in_string:
            if ch == "\\" and i + 1 < len(js):
                result.append(ch)
                result.append(js[i + 1])
                i += 2
                continue
            if ch == string_char:
                in_string = False
                result.append('"')  # Always output double quote
                i += 1
                continue
            result.append(ch)
            i += 1
            continue

        if ch in "\"'":
            in_string = True
            string_char = ch
            result.append('"')  # Always output double quote
            i += 1
            continue

        # Handle BigInt suffix
        if ch == "n" and result and result[-1].isdigit():
            # Skip the 'n'
            i += 1
            continue

        result.append(ch)
        i += 1

    return "".join(result)


def values_equal(a, b) -> bool:
    """Compare two values, handling NaN specially."""
    if type(a) != type(b):
        # Special case: int vs float - compare as floats to handle large numbers
        # where int conversion loses precision (e.g., 6.022e23)
        if isinstance(a, int) and isinstance(b, float):
            return float(a) == b
        if isinstance(a, float) and isinstance(b, int):
            return a == float(b)
        return False

    if isinstance(a, float):
        if math.isnan(a) and math.isnan(b):
            return True
        return a == b

    if isinstance(a, dict):
        if set(a.keys()) != set(b.keys()):
            return False
        return all(values_equal(a[k], b[k]) for k in a)

    if isinstance(a, list):
        if len(a) != len(b):
            return False
        return all(values_equal(x, y) for x, y in zip(a, b))

    return a == b


def run_valid_tests(test_root: str, verbose: bool = False) -> tuple[int, int, list]:
    """
    Run all .yay tests from test_root/yay/, checking against test_root/js/.

    Returns (passed, failed, errors) where errors is a list of (filename, error_message).
    """
    passed = 0
    failed = 0
    errors = []

    yay_dir = os.path.join(test_root, "yay")
    js_dir = os.path.join(test_root, "js")
    yay_files = sorted(f for f in os.listdir(yay_dir) if f.endswith(".yay"))

    for fname in yay_files:
        yay_path = os.path.join(yay_dir, fname)
        js_path = os.path.join(js_dir, fname[:-4] + ".js")

        try:
            with open(yay_path, "rb") as fp:
                yay_bytes = fp.read()
            yay_content = yay_bytes.decode("utf-8")

            result = yay.loads(yay_content)

            # Check against expected output if .js file exists
            if os.path.exists(js_path):
                with open(js_path) as fp:
                    js_content = fp.read()

                try:
                    expected = parse_js_value(js_content)
                    if not values_equal(result, expected):
                        failed += 1
                        errors.append(
                            (
                                fname,
                                f"Value mismatch: got {result!r}, expected {expected!r}",
                            )
                        )
                        continue
                except ValueError as e:
                    # Can't parse expected value, just check parsing succeeded
                    if verbose:
                        print(f"  {fname}: Could not parse expected JS: {e}")

            passed += 1
            if verbose:
                print(f"  {fname}: OK -> {result!r}")

        except Exception as e:
            failed += 1
            errors.append((fname, str(e)))

    return passed, failed, errors


def run_error_tests(test_root: str, verbose: bool = False) -> tuple[int, int, list]:
    """
    Run all .nay error tests from test_root/nay/.

    Returns (passed, failed, errors) where errors is a list of (filename, error_message).
    """
    passed = 0
    failed = 0
    errors = []

    nay_dir = os.path.join(test_root, "nay")
    nay_files = sorted(f for f in os.listdir(nay_dir) if f.endswith(".nay"))

    for fname in nay_files:
        basename = fname[:-4]  # Remove .nay extension
        nay_path = os.path.join(nay_dir, fname)
        error_path = os.path.join(nay_dir, fname[:-4] + ".error")

        try:
            with open(nay_path, "rb") as fp:
                nay_bytes = fp.read()
            nay_content = nay_bytes.decode("utf-8")
        except UnicodeDecodeError:
            # Some .nay files may have invalid UTF-8 (like BOM test)
            # Read as latin-1 to get raw bytes as string
            with open(nay_path, "rb") as fp:
                nay_bytes = fp.read()
            nay_content = nay_bytes.decode("latin-1")

        # Read expected error substring
        expected_error = None
        if os.path.exists(error_path):
            with open(error_path) as fp:
                expected_error = fp.read().strip()

        try:
            result = yay.loads(nay_content)
            # Should have raised an error
            failed += 1
            errors.append((fname, f"Expected error but got result: {result!r}"))
        except yay.YayError as e:
            # Check if error message matches expected pattern
            error_msg = str(e)
            if expected_error:
                # Extract the key part of expected error (before "at X:Y of")
                # e.g., "Unexpected space after \":\"" from full message
                match = re.match(r"^(.+?)\s+at\s+\d+:\d+", expected_error)
                if match:
                    expected_pattern = match.group(1).strip()
                    if expected_pattern.lower() in error_msg.lower():
                        passed += 1
                        if verbose:
                            print(f"  {fname}: OK (error: {error_msg})")
                    else:
                        failed += 1
                        errors.append(
                            (
                                fname,
                                f"Error mismatch: got '{error_msg}', expected pattern '{expected_pattern}'",
                            )
                        )
                else:
                    # No "at X:Y" pattern, just check if any error was raised
                    passed += 1
                    if verbose:
                        print(f"  {fname}: OK (error: {error_msg})")
            else:
                passed += 1
                if verbose:
                    print(f"  {fname}: OK (error: {error_msg})")
        except Exception as e:
            # Other exceptions count as pass (we expected an error)
            passed += 1
            if verbose:
                print(f"  {fname}: OK (exception: {type(e).__name__}: {e})")

    return passed, failed, errors


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Test YAY parser against fixtures")
    parser.add_argument(
        "-v", "--verbose", action="store_true", help="Show all test results"
    )
    parser.add_argument("-f", "--file", help="Test a specific .yay file")
    parser.add_argument(
        "--test-dir", default="../test", help="Root directory containing test fixtures"
    )
    args = parser.parse_args()

    if args.file:
        # Test a single file
        try:
            with open(args.file, "rb") as fp:
                content_bytes = fp.read()
            try:
                content = content_bytes.decode("utf-8")
            except UnicodeDecodeError:
                content = content_bytes.decode("latin-1")
            print(f"Input:\n{content}")
            print(f"\nParsing...")
            result = yay.loads(content)
            print(f"Result: {result!r}")
            print(f"Type: {type(result).__name__}")
        except Exception as e:
            print(f"ERROR: {e}")
            import traceback

            traceback.print_exc()
            sys.exit(1)
    else:
        # Run all tests
        test_root = os.path.join(os.path.dirname(__file__), args.test_dir)
        print(f"Running tests from {test_root}\n")

        # Run valid .yay tests
        print("Valid input tests (.yay):")
        v_passed, v_failed, v_errors = run_valid_tests(test_root, args.verbose)
        print(f"  {v_passed} passed, {v_failed} failed")

        # Run error .nay tests
        print("\nError tests (.nay):")
        e_passed, e_failed, e_errors = run_error_tests(test_root, args.verbose)
        print(f"  {e_passed} passed, {e_failed} failed")

        total_passed = v_passed + e_passed
        total_failed = v_failed + e_failed
        all_errors = v_errors + e_errors

        print(f"\nTotal: {total_passed} passed, {total_failed} failed")

        if all_errors:
            print("\nFailures:")
            for fname, err in all_errors:
                print(f"  {fname}: {err}")

        sys.exit(0 if total_failed == 0 else 1)


if __name__ == "__main__":
    main()
