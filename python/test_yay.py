#!/usr/bin/env python3
"""
Test harness for the YAY parser.

Runs against all .yay fixture files and compares with expected .py output.
Runs against all .nay fixture files and verifies errors are raised.
"""

import ast
import math
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
import libyay as yay


def parse_py_value(py_content: str):
    """
    Evaluate an expected-value fixture from test/py/.

    The fixtures are Python literals, plus calls to bytes.fromhex("..."),
    float("inf"), float("-inf") or float("nan"), Tagged("tag", payload) and
    Reference("uri").
    Nothing else is evaluated.
    """

    def evaluate(node):
        if (
            isinstance(node, ast.Call)
            and ast.unparse(node.func) == "Tagged"
            and len(node.args) == 2
            and not node.keywords
        ):
            tag = ast.literal_eval(node.args[0])
            if isinstance(tag, str):
                return yay.Tagged(tag, evaluate(node.args[1]))
        if (
            isinstance(node, ast.Call)
            and ast.unparse(node.func) == "Reference"
            and len(node.args) == 1
            and not node.keywords
        ):
            uri = ast.literal_eval(node.args[0])
            if isinstance(uri, str):
                return yay.Reference(uri)
        if isinstance(node, ast.Call) and len(node.args) == 1 and not node.keywords:
            func = ast.unparse(node.func)
            arg = ast.literal_eval(node.args[0])
            if func == "bytes.fromhex" and isinstance(arg, str):
                return bytes.fromhex(arg)
            if func == "float" and arg in ("inf", "-inf", "nan"):
                return float(arg)
            raise ValueError(f"Unexpected call in fixture: {ast.unparse(node)}")
        if isinstance(node, ast.List):
            return [evaluate(item) for item in node.elts]
        if isinstance(node, ast.Dict):
            return {
                ast.literal_eval(key): evaluate(value)
                for key, value in zip(node.keys, node.values)
            }
        return ast.literal_eval(node)

    return evaluate(ast.parse(py_content.strip(), mode="eval").body)


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

    if isinstance(a, yay.Tagged):
        return a.tag == b.tag and values_equal(a.payload, b.payload)

    if isinstance(a, list):
        if len(a) != len(b):
            return False
        return all(values_equal(x, y) for x, y in zip(a, b))

    return a == b


def run_valid_tests(test_root: str, verbose: bool = False) -> tuple[int, int, list]:
    """
    Run all .yay tests from test_root/yay/, checking against test_root/py/.

    Returns (passed, failed, errors) where errors is a list of (filename, error_message).
    """
    passed = 0
    failed = 0
    errors = []

    yay_dir = os.path.join(test_root, "yay")
    py_dir = os.path.join(test_root, "py")
    yay_files = sorted(f for f in os.listdir(yay_dir) if f.endswith(".yay"))

    for fname in yay_files:
        yay_path = os.path.join(yay_dir, fname)
        py_path = os.path.join(py_dir, fname[:-4] + ".py")

        try:
            with open(yay_path, "rb") as fp:
                yay_bytes = fp.read()
            yay_content = yay_bytes.decode("utf-8")

            result = yay.loads(yay_content)

            # Check against expected output if .py file exists
            if os.path.exists(py_path):
                with open(py_path) as fp:
                    expected = parse_py_value(fp.read())
                if not values_equal(result, expected):
                    failed += 1
                    errors.append(
                        (
                            fname,
                            f"Value mismatch: got {result!r}, expected {expected!r}",
                        )
                    )
                    continue

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
            # The whole message, its line and column included, in this port's
            # form: "(line L, col C)" for " at L:C of <file>", and nothing for
            # the file name of a message without a location
            error_msg = str(e)
            if expected_error is not None:
                file = re.escape(f"<{fname}>")
                expected_msg = re.sub(
                    rf" at (\d+):(\d+) of {file}$",
                    r" (line \1, col \2)",
                    expected_error,
                )
                expected_msg = re.sub(rf" {file}$", "", expected_msg)
                if error_msg == expected_msg:
                    passed += 1
                    if verbose:
                        print(f"  {fname}: OK (error: {error_msg})")
                else:
                    failed += 1
                    errors.append(
                        (
                            fname,
                            f"Error mismatch: got '{error_msg}', expected '{expected_msg}'",
                        )
                    )
            else:
                passed += 1
                if verbose:
                    print(f"  {fname}: OK (error: {error_msg})")
        except Exception as e:
            # Any other exception is a defect, not the expected error
            failed += 1
            errors.append((fname, f"Unexpected {type(e).__name__}: {e}"))

    return passed, failed, errors


# Documents the shared fixtures do not hold, each with its value, or None
# where it must be refused.
PORT_CASES = [
    # GRAMMAR.md's nested-content: a block string opened below its key with
    # text on the backtick's line reads as at the root (a backtick alone
    # there is test/nay's string-block-property-invalid-leader-next-line)
    ("key:\n  ` First line.\n    more\n", {"key": "First line.\nmore\n"}),
    ("key:\n  ` text\nnext: 1\n", {"key": "text\n", "next": 1}),
    ("a:\n  b:\n    ` x\n  c: 1\n", {"a": {"b": "x\n", "c": 1}}),
    ("key:\n  ` x\n  y: 1\n", None),
]


def run_port_cases(verbose: bool = False) -> tuple[int, int, list]:
    """Run PORT_CASES. Returns (passed, failed, errors)."""
    passed = 0
    failed = 0
    errors = []
    for source, expected in PORT_CASES:
        try:
            result = yay.loads(source)
        except yay.YayError as e:
            if expected is None:
                passed += 1
            else:
                failed += 1
                errors.append((repr(source), str(e)))
            continue
        if expected is not None and values_equal(result, expected):
            passed += 1
            if verbose:
                print(f"  {source!r}: OK -> {result!r}")
        else:
            failed += 1
            errors.append((repr(source), f"got {result!r}, expected {expected!r}"))
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

        # Run the port's own cases
        print("\nPort cases:")
        p_passed, p_failed, p_errors = run_port_cases(args.verbose)
        print(f"  {p_passed} passed, {p_failed} failed")

        total_passed = v_passed + e_passed + p_passed
        total_failed = v_failed + e_failed + p_failed
        all_errors = v_errors + e_errors + p_errors

        print(f"\nTotal: {total_passed} passed, {total_failed} failed")

        if all_errors:
            print("\nFailures:")
            for fname, err in all_errors:
                print(f"  {fname}: {err}")

        sys.exit(0 if total_failed == 0 else 1)


if __name__ == "__main__":
    main()
