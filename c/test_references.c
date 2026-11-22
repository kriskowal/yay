/*
 * Reference tests
 *
 * MEH's leniency is not YAY's, so the corpus's MEH reference cases,
 * ../test/meh/reference-*.meh, are refused as YAY. The corpus reference
 * cases in test/yay and test/nay are read with the rest of the corpus.
 */

#define _DEFAULT_SOURCE
#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "yay.h"

#define MEH "../test/meh"

static int failures = 0;
static int checks = 0;

static void check(bool ok, const char *what) {
    checks++;
    if (!ok) {
        failures++;
        printf("FAIL: %s\n", what);
    }
}

static char *read_file(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *buf = malloc((size_t)n + 1);
    size_t got = fread(buf, 1, (size_t)n, f);
    buf[got] = '\0';
    fclose(f);
    return buf;
}

static bool has_ext(const char *name, const char *ext) {
    size_t n = strlen(name), e = strlen(ext);
    return n > e && strcmp(name + n - e, ext) == 0;
}

static void test_meh_cases(void) {
    DIR *dir = opendir(MEH);
    check(dir != NULL, "test/meh directory");
    if (!dir) return;
    int refused = 0;
    struct dirent *ent;
    while ((ent = readdir(dir))) {
        const char *name = ent->d_name;
        if (strncmp(name, "reference-", 10) != 0 || !has_ext(name, ".meh")) continue;
        char path[1024];
        snprintf(path, sizeof(path), "%s/%s", MEH, name);
        char *source = read_file(path);
        yay_result_t result = yay_parse(source, 0, name);
        char what[1200];
        refused++;
        snprintf(what, sizeof(what), "%s: accepted, should be refused", name);
        check(result.error != NULL, what);
        yay_result_free(&result);
        free(source);
    }
    closedir(dir);
    check(refused >= 2, "MEH reference cases read");
    printf("MEH reference cases: %d refused\n", refused);
}

static yay_value_t *parse_ok(const char *source) {
    yay_result_t result = yay_parse(source, 0, NULL);
    if (result.error) {
        printf("  %s: %s\n", source, result.error->message);
        yay_error_free(result.error);
        return NULL;
    }
    return result.value;
}

/* The Rust, JavaScript, Python and Go ports' messages and columns */
static void test_refused(void) {
    const char *cases[][2] = {
        {"a: <HTTPS://x>\n", "Uppercase scheme \"HTTPS\" in reference (use lowercase) at 1:4 of <t>"},
        {"a: <https://x y>\n", "Unexpected space in reference at 1:14 of <t>"},
        {"a: <https://x\"y>\n", "Unexpected character \"\"\" in reference at 1:14 of <t>"},
        {"a: <https://x/%2g>\n", "Bad percent-encoding in reference (use % and two hex digits) at 1:15 of <t>"},
        {"a: <https://x/%2>\n", "Bad percent-encoding in reference (use % and two hex digits) at 1:15 of <t>"},
        {"a: <https://x\n", "Unmatched angle bracket at 1:4 of <t>"},
        {"a: <https://x>(t)\n", "Expected one space before a tag at 1:15 of <t>"},
        {"a: <https://x>rgb\n", "Unexpected character \"r\" at 1:15 of <t>"},
        {"<https://x>: 1\n", "Unexpected character \":\" at 1:12 of <t>"},
    };
    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        yay_result_t result = yay_parse(cases[i][0], 0, "t");
        char what[512];
        snprintf(what, sizeof(what), "%s: got %s, want %s", cases[i][0],
                 result.error ? result.error->message : "a value", cases[i][1]);
        check(result.error && strcmp(result.error->message, cases[i][1]) == 0, what);
        yay_result_free(&result);
    }
}

static void test_model(void) {
    yay_value_t *a = yay_reference("https://x");
    yay_value_t *b = yay_reference_len("https://x/", 9);
    yay_value_t *slash = yay_reference("https://x/");
    yay_value_t *lower = yay_reference("a:%2f");
    yay_value_t *upper = yay_reference("a:%2F");
    yay_value_t *string = yay_string("https://x");
    yay_value_t *tagged = yay_tagged("https://x", yay_null());
    check(a->type == YAY_REFERENCE, "type");
    check(strcmp(a->data.reference, "https://x") == 0, "text");
    check(yay_equal(a, b), "equal by text");
    check(!yay_equal(a, slash), "texts differ");
    check(!yay_equal(lower, upper), "percent-encodings are not normalized");
    check(!yay_equal(a, string) && !yay_equal(string, a), "never a string");
    check(!yay_equal(a, tagged), "never a tagged value");
    char *str = yay_to_string(a);
    check(strcmp(str, "<https://x>") == 0, "to string");
    free(str);
    yay_free(a);
    yay_free(b);
    yay_free(slash);
    yay_free(lower);
    yay_free(upper);
    yay_free(string);
    yay_free(tagged);
}

static void test_parse(void) {
    yay_value_t *v = parse_ok("<ocapn://a.tcp> (Counter)\n");
    yay_value_t *want = yay_tagged("Counter", yay_reference("ocapn://a.tcp"));
    check(v && yay_equal(v, want), "a tagged reference at the root");
    yay_free(v);
    yay_free(want);

    /* Unchanged in strings and tags, and bytes without a scheme */
    v = parse_ok("a: '<https://x>' (<b:c>)\nb: <ff>\n");
    want = YAY_OBJECT("a", yay_tagged("<b:c>", yay_string("<https://x>")),
                      "b", yay_bytes_from_hex("ff"));
    check(v && yay_equal(v, want), "strings, tags and bytes");
    yay_free(v);
    yay_free(want);

    /* A reference's punctuation is its own */
    v = parse_ok("a: [<x:a,b]c#d>, 1]  # c\n");
    want = YAY_OBJECT("a", YAY_ARRAY(yay_reference("x:a,b]c#d"), yay_int(1)));
    check(v && yay_equal(v, want), "punctuation in an inline array");
    yay_free(v);
    yay_free(want);

    v = parse_ok("a: {b: <x:(t)>} (u)\n");
    want = YAY_OBJECT("a", yay_tagged("u", YAY_OBJECT("b", yay_reference("x:(t)"))));
    check(v && yay_equal(v, want), "punctuation in an inline object");
    yay_free(v);
    yay_free(want);
}

/* A reference is made only from a reference's text */
static void test_constructor(void) {
    const char *bad[] = {"", "not a uri", "HTTPS://x", "urn:a b",
                         "https://a>b", "https://x/%zz", "./a.yay"};
    for (size_t i = 0; i < sizeof(bad) / sizeof(bad[0]); i++) {
        char what[64];
        snprintf(what, sizeof(what), "yay_reference(\"%s\") refused", bad[i]);
        yay_value_t *r = yay_reference(bad[i]);
        check(r == NULL, what);
        yay_free(r);
        r = yay_reference_len(bad[i], strlen(bad[i]));
        check(r == NULL, what);
        yay_free(r);
    }
    /* A NUL inside the text, and text cut short of its percent-encoding */
    yay_value_t *r = yay_reference_len("a:b\0c", 5);
    check(r == NULL, "a NUL in a reference refused");
    yay_free(r);
    r = yay_reference_len("a:%2f", 4);
    check(r == NULL, "a percent-encoding cut short refused");
    yay_free(r);
    const char *good[] = {"urn:x", "https://example.com/a?b=c#d", "urn:%C3%A9",
                          "urn:\xC3\xA9"};
    for (size_t i = 0; i < sizeof(good) / sizeof(good[0]); i++) {
        yay_value_t *r = yay_reference(good[i]);
        check(r && r->type == YAY_REFERENCE &&
                  strcmp(r->data.reference, good[i]) == 0,
              good[i]);
        yay_free(r);
    }
}

int main(void) {
    test_constructor();
    test_model();
    test_refused();
    test_parse();
    test_meh_cases();
    printf("references: %d/%d checks passed\n", checks - failures, checks);
    return failures ? 1 : 0;
}
