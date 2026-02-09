/*
 * YAY Parser Test Runner
 *
 * This program runs all test fixtures and compares parsed results
 * against expected values. It also tests error cases.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

#include "yay.h"
#include "fixtures_gen.h"

/* ANSI color codes */
#define COLOR_RED     "\x1b[31m"
#define COLOR_GREEN   "\x1b[32m"
#define COLOR_YELLOW  "\x1b[33m"
#define COLOR_RESET   "\x1b[0m"

/* Test statistics */
static int tests_run = 0;
static int tests_passed = 0;
static int tests_failed = 0;

/* Error test statistics */
static int error_tests_run = 0;
static int error_tests_passed = 0;
static int error_tests_failed = 0;

/* Print a value diff for debugging */
static void print_value_diff(const char *label, const yay_value_t *v) {
    char *str = yay_to_string(v);
    printf("  %s: %s\n", label, str ? str : "(null)");
    free(str);
}

/* Run a single valid test fixture */
static bool run_test(const test_fixture_t *fixture) {
    tests_run++;
    
    printf("Testing: %s ... ", fixture->name);
    fflush(stdout);
    
    /* Parse the YAY source */
    yay_result_t result = yay_parse(fixture->yay_source, 0, fixture->name);
    
    if (result.error) {
        printf(COLOR_RED "FAIL" COLOR_RESET " (parse error)\n");
        printf("  Error: %s\n", result.error->message);
        yay_result_free(&result);
        tests_failed++;
        return false;
    }
    
    /* Get expected value */
    yay_value_t *expected = fixture->make_expected();
    
    /* Compare */
    bool equal = yay_equal(result.value, expected);
    
    if (equal) {
        printf(COLOR_GREEN "PASS" COLOR_RESET "\n");
        tests_passed++;
    } else {
        printf(COLOR_RED "FAIL" COLOR_RESET " (value mismatch)\n");
        print_value_diff("Expected", expected);
        print_value_diff("Got     ", result.value);
        tests_failed++;
    }
    
    /* Cleanup */
    yay_free(expected);
    yay_result_free(&result);
    
    return equal;
}

/* Run a single error test fixture */
static bool run_error_test(const error_fixture_t *fixture) {
    error_tests_run++;
    
    printf("Testing: %s ... ", fixture->name);
    fflush(stdout);
    
    /* Parse the invalid YAY source - should fail */
    /* Use original_name to match expected error message format */
    yay_result_t result = yay_parse(fixture->nay_source, fixture->nay_len, fixture->original_name);
    
    if (!result.error) {
        printf(COLOR_RED "FAIL" COLOR_RESET " (expected error, got success)\n");
        char *str = yay_to_string(result.value);
        printf("  Got value: %s\n", str ? str : "(null)");
        free(str);
        yay_result_free(&result);
        error_tests_failed++;
        return false;
    }
    
    /* Check if error message contains expected pattern */
    if (strstr(result.error->message, fixture->error_pattern) != NULL) {
        printf(COLOR_GREEN "PASS" COLOR_RESET "\n");
        error_tests_passed++;
        yay_result_free(&result);
        return true;
    } else {
        printf(COLOR_RED "FAIL" COLOR_RESET " (error message mismatch)\n");
        printf("  Expected pattern: %s\n", fixture->error_pattern);
        printf("  Got: %s\n", result.error->message);
        yay_result_free(&result);
        error_tests_failed++;
        return false;
    }
}

/* Run all tests */
static void run_all_tests(void) {
    printf("\n");
    printf("========================================\n");
    printf("  YAY Parser C Test Suite\n");
    printf("========================================\n\n");
    
    printf("--- Valid Input Tests (.yay) ---\n\n");
    for (int i = 0; test_fixtures[i].name != NULL; i++) {
        run_test(&test_fixtures[i]);
    }
    
    printf("\n--- Error Tests (.nay) ---\n\n");
    for (int i = 0; error_fixtures[i].name != NULL; i++) {
        run_error_test(&error_fixtures[i]);
    }
    
    printf("\n");
    printf("========================================\n");
    printf("  Valid tests: %d/%d passed", tests_passed, tests_run);
    if (tests_failed > 0) {
        printf(" (" COLOR_RED "%d failed" COLOR_RESET ")", tests_failed);
    }
    printf("\n");
    printf("  Error tests: %d/%d passed", error_tests_passed, error_tests_run);
    if (error_tests_failed > 0) {
        printf(" (" COLOR_RED "%d failed" COLOR_RESET ")", error_tests_failed);
    }
    printf("\n");
    printf("  Total: %d/%d passed", tests_passed + error_tests_passed, tests_run + error_tests_run);
    if (tests_failed + error_tests_failed > 0) {
        printf(" (" COLOR_RED "%d failed" COLOR_RESET ")", tests_failed + error_tests_failed);
    }
    printf("\n");
    printf("========================================\n\n");
}

/* Run a specific test by name */
static bool run_named_test(const char *name) {
    for (int i = 0; test_fixtures[i].name != NULL; i++) {
        if (strcmp(test_fixtures[i].name, name) == 0) {
            return run_test(&test_fixtures[i]);
        }
    }
    
    printf("Unknown test: %s\n", name);
    return false;
}

/* List all available tests */
static void list_tests(void) {
    printf("Available tests:\n");
    for (int i = 0; test_fixtures[i].name != NULL; i++) {
        printf("  %s\n", test_fixtures[i].name);
    }
    printf("\nTotal: %d tests\n", TEST_FIXTURE_COUNT);
}

/* Parse and print a YAY file (for debugging) */
static void parse_file(const char *filename) {
    FILE *f = fopen(filename, "r");
    if (!f) {
        fprintf(stderr, "Cannot open file: %s\n", filename);
        return;
    }
    
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    
    char *content = malloc(size + 1);
    size_t nread = fread(content, 1, size, f);
    content[nread] = '\0';
    fclose(f);
    
    yay_result_t result = yay_parse(content, size, filename);
    
    if (result.error) {
        fprintf(stderr, "Parse error: %s\n", result.error->message);
    } else {
        char *str = yay_to_string(result.value);
        printf("%s\n", str);
        free(str);
    }
    
    yay_result_free(&result);
    free(content);
}

/* Print usage */
static void usage(const char *prog) {
    printf("Usage: %s [options]\n", prog);
    printf("\n");
    printf("Options:\n");
    printf("  (no args)       Run all tests\n");
    printf("  -l, --list      List all available tests\n");
    printf("  -t, --test NAME Run a specific test by name\n");
    printf("  -f, --file FILE Parse a YAY file and print result\n");
    printf("  -h, --help      Show this help\n");
}

int main(int argc, char *argv[]) {
    if (argc == 1) {
        /* No arguments - run all tests */
        run_all_tests();
        return (tests_failed + error_tests_failed) > 0 ? 1 : 0;
    }
    
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            usage(argv[0]);
            return 0;
        }
        else if (strcmp(argv[i], "-l") == 0 || strcmp(argv[i], "--list") == 0) {
            list_tests();
            return 0;
        }
        else if (strcmp(argv[i], "-t") == 0 || strcmp(argv[i], "--test") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "Missing test name\n");
                return 1;
            }
            bool passed = run_named_test(argv[++i]);
            return passed ? 0 : 1;
        }
        else if (strcmp(argv[i], "-f") == 0 || strcmp(argv[i], "--file") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "Missing filename\n");
                return 1;
            }
            parse_file(argv[++i]);
            return 0;
        }
        else {
            fprintf(stderr, "Unknown option: %s\n", argv[i]);
            usage(argv[0]);
            return 1;
        }
    }
    
    return 0;
}
