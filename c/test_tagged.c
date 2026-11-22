/*
 * Tagged value tests
 *
 * Reads the design's cases in ../doc/design/tagged-values-g/cases/: each
 * .yay parses to its .json, with a tagged value as {"#tag": tag, "payload":
 * value} and bytes as "*hex"; each .nay and .meh is refused as YAY.
 */

#define _DEFAULT_SOURCE
#include <dirent.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "yay.h"

#define CASES "../doc/design/tagged-values-g/cases"

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

/* A small JSON reader for the expected values, desugaring tags and bytes */

static yay_value_t *json_value(const char **p);

static void json_space(const char **p) {
    while (**p == ' ' || **p == '\n' || **p == '\t' || **p == '\r') (*p)++;
}

static char *json_string(const char **p) {
    size_t cap = strlen(*p) + 1;
    char *buf = malloc(cap);
    size_t n = 0;
    (*p)++;
    while (**p != '"') {
        if (**p == '\\') {
            (*p)++;
            char e = **p;
            if (e == 'n') buf[n++] = '\n';
            else if (e == 't') buf[n++] = '\t';
            else if (e == 'u') {
                unsigned cp = (unsigned)strtoul((char[]){(*p)[1], (*p)[2], (*p)[3], (*p)[4], 0}, NULL, 16);
                *p += 4;
                if (cp < 0x80) {
                    buf[n++] = (char)cp;
                } else if (cp < 0x800) {
                    buf[n++] = (char)(0xC0 | (cp >> 6));
                    buf[n++] = (char)(0x80 | (cp & 0x3F));
                } else {
                    buf[n++] = (char)(0xE0 | (cp >> 12));
                    buf[n++] = (char)(0x80 | ((cp >> 6) & 0x3F));
                    buf[n++] = (char)(0x80 | (cp & 0x3F));
                }
            } else buf[n++] = e;
            (*p)++;
        } else {
            buf[n++] = *(*p)++;
        }
    }
    (*p)++;
    buf[n] = '\0';
    return buf;
}

static yay_value_t *json_value(const char **p) {
    json_space(p);
    if (**p == '{') {
        (*p)++;
        yay_value_t *obj = yay_object();
        json_space(p);
        while (**p != '}') {
            char *key = json_string(p);
            json_space(p);
            (*p)++; /* : */
            yay_object_set(obj, key, json_value(p));
            free(key);
            json_space(p);
            if (**p == ',') (*p)++;
            json_space(p);
        }
        (*p)++;
        if (obj->data.object.length == 2 &&
            strcmp(obj->data.object.pairs[0].key, "#tag") == 0 &&
            strcmp(obj->data.object.pairs[1].key, "payload") == 0) {
            yay_value_t *tagged = yay_tagged(
                obj->data.object.pairs[0].value->data.string,
                obj->data.object.pairs[1].value);
            obj->data.object.pairs[1].value = NULL;
            yay_free(obj);
            return tagged;
        }
        return obj;
    }
    if (**p == '[') {
        (*p)++;
        yay_value_t *arr = yay_array();
        json_space(p);
        while (**p != ']') {
            yay_array_push(arr, json_value(p));
            json_space(p);
            if (**p == ',') (*p)++;
            json_space(p);
        }
        (*p)++;
        return arr;
    }
    if (**p == '"') {
        char *s = json_string(p);
        /* The cases' JSON spells NaN as a string */
        yay_value_t *v = s[0] == '*' ? yay_bytes_from_hex(s + 1)
                       : strcmp(s, "NaN") == 0 ? yay_float(NAN)
                       : yay_string(s);
        free(s);
        return v;
    }
    if (strncmp(*p, "null", 4) == 0) { *p += 4; return yay_null(); }
    if (strncmp(*p, "true", 4) == 0) { *p += 4; return yay_bool(true); }
    if (strncmp(*p, "false", 5) == 0) { *p += 5; return yay_bool(false); }
    if (strncmp(*p, "NaN", 3) == 0) { *p += 3; return yay_float(NAN); }
    if (strncmp(*p, "Infinity", 8) == 0) { *p += 8; return yay_float(INFINITY); }
    if (strncmp(*p, "-Infinity", 9) == 0) { *p += 9; return yay_float(-INFINITY); }
    const char *start = *p;
    bool is_float = false;
    while (strchr("+-0123456789.eE", **p) && **p) {
        if (**p == '.' || **p == 'e' || **p == 'E') is_float = true;
        (*p)++;
    }
    char *num = strndup(start, (size_t)(*p - start));
    yay_value_t *v = is_float ? yay_float(strtod(num, NULL))
                   : num[0] == '-' ? yay_int_from_str(num + 1, true)
                   : yay_int_from_str(num, false);
    free(num);
    return v;
}

static bool has_ext(const char *name, const char *ext) {
    size_t n = strlen(name), e = strlen(ext);
    return n > e && strcmp(name + n - e, ext) == 0;
}

static void test_cases(void) {
    DIR *dir = opendir(CASES);
    check(dir != NULL, "cases directory");
    if (!dir) return;
    int accepted = 0, refused = 0;
    struct dirent *ent;
    while ((ent = readdir(dir))) {
        const char *name = ent->d_name;
        bool yay = has_ext(name, ".yay");
        bool nay = has_ext(name, ".nay") || has_ext(name, ".meh");
        if (!yay && !nay) continue;
        char path[1024];
        snprintf(path, sizeof(path), "%s/%s", CASES, name);
        char *source = read_file(path);
        yay_result_t result = yay_parse(source, 0, name);
        char what[1200];
        if (yay) {
            accepted++;
            snprintf(path + strlen(path) - 4, 6, ".json");
            char *json = read_file(path);
            const char *p = json;
            yay_value_t *expected = json_value(&p);
            bool ok = !result.error && yay_equal(result.value, expected);
            snprintf(what, sizeof(what), "%s: %s", name,
                     result.error ? result.error->message : "value differs");
            check(ok, what);
            if (!ok && !result.error) {
                char *got = yay_to_string(result.value);
                char *want = yay_to_string(expected);
                printf("  got:  %s\n  want: %s\n", got, want);
                free(got);
                free(want);
            }
            yay_free(expected);
            free(json);
        } else {
            refused++;
            snprintf(what, sizeof(what), "%s: accepted, should be refused", name);
            check(result.error != NULL, what);
        }
        yay_result_free(&result);
        free(source);
    }
    closedir(dir);
    check(accepted > 0 && refused > 0, "cases read");
    printf("cases: %d accepted, %d refused\n", accepted, refused);
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
    test_cases();
    printf("tagged: %d/%d checks passed\n", checks - failures, checks);
    return failures ? 1 : 0;
}
