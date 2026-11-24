/*
 * Tagged value tests
 *
 * MEH's leniency is not YAY's, so the corpus's MEH tagged cases,
 * ../test/meh/tagged-*.meh, are refused as YAY. The corpus tagged cases in
 * test/yay and test/nay are read with the rest of the corpus.
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
        if (strncmp(name, "tagged-", 7) != 0 || !has_ext(name, ".meh")) continue;
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
    check(refused >= 3, "MEH tagged cases read");
    printf("MEH tagged cases: %d refused\n", refused);
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

static void test_model(void) {
    yay_value_t *a = yay_tagged("t", yay_int(1));
    yay_value_t *b = yay_tagged("t", yay_int(1));
    yay_value_t *c = yay_tagged("u", yay_int(1));
    yay_value_t *d = yay_tagged("t", yay_int(2));
    yay_value_t *one = yay_int(1);
    check(a->type == YAY_TAGGED, "type");
    check(strcmp(a->data.tagged.tag, "t") == 0, "tag");
    check(yay_equal(a, b), "equal tags and payloads are equal");
    check(!yay_equal(a, c), "tags differ");
    check(!yay_equal(a, d), "payloads differ");
    check(!yay_equal(a, one) && !yay_equal(one, a), "never its payload");
    char *str = yay_to_string(a);
    check(strcmp(str, "1n (t)") == 0, "to string");
    free(str);
    yay_free(a);
    yay_free(b);
    yay_free(c);
    yay_free(d);
    yay_free(one);
}

static void test_escapes_and_nesting(void) {
    yay_value_t *v = parse_ok("a: 1 (x\\u{41}\\\\\\n) (outer)\n");
    yay_value_t *want = YAY_OBJECT("a",
        yay_tagged("outer", yay_tagged("xA\\\n", yay_int(1))));
    check(v && yay_equal(v, want), "escapes and nesting innermost first");
    yay_free(v);
    yay_free(want);

    const char *refused[] = {
        "a: 1 (x\\\"y)\n", "a: 1 (x\\(y)\n", "a: 1 (x\n", "a: 1  (t)\n",
        "a: 1 (t)  b\n", "a: (t)\n", "- (t) 1\n", "(t)\n1\n",
        "a: [1(t)]\n", "a: {b: 'x'(t)}\n",
    };
    for (size_t i = 0; i < sizeof(refused) / sizeof(refused[0]); i++) {
        yay_result_t result = yay_parse(refused[i], 0, NULL);
        char what[128];
        snprintf(what, sizeof(what), "refused: %s", refused[i]);
        check(result.error != NULL, what);
        yay_result_free(&result);
    }

    v = parse_ok("a: 1 (t)  # note\n");
    want = YAY_OBJECT("a", yay_tagged("t", yay_int(1)));
    check(v && yay_equal(v, want), "a comment after a tag");
    yay_free(v);
    yay_free(want);

    v = parse_ok("- (a)\n  - (b)\n    k: 'v'\n");
    want = YAY_ARRAY(yay_tagged("a", YAY_ARRAY(
        yay_tagged("b", YAY_OBJECT("k", yay_string("v"))))));
    check(v && yay_equal(v, want), "tagged blocks within tagged blocks");
    yay_free(v);
    yay_free(want);
}

int main(void) {
    test_model();
    test_escapes_and_nesting();
    test_meh_cases();
    printf("tagged: %d/%d checks passed\n", checks - failures, checks);
    return failures ? 1 : 0;
}
