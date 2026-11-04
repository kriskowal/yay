/*
 * Reference tests
 *
 * Reads the design's cases in ../doc/design/references/cases/: each .yay
 * parses to its .yson, with a reference spelled "$uri", and each .nay and
 * .meh is refused, MEH's leniency not being YAY's.
 */

#define _DEFAULT_SOURCE
#include <dirent.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "yay.h"

#define CASES "../doc/design/references/cases"

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

/* A small JSON reader for the expected values, desugaring YSON's strings:
 * "$uri" a reference, "*hex" bytes, "+n" and "-n" integers, "#nan" and the
 * infinities floats, and "!" before text that would read as one of those */

static yay_value_t *ysonString(const char *s) {
    switch (s[0]) {
        case '$': return yay_reference(s + 1);
        case '*': return yay_bytes_from_hex(s + 1);
        case '+': return yay_int_from_str(s + 1, false);
        case '-': return yay_int_from_str(s + 1, true);
        case '!': return yay_string(s + 1);
        case '#':
            if (strcmp(s, "#nan") == 0) return yay_float(NAN);
            if (strcmp(s, "#+inf") == 0) return yay_float(INFINITY);
            if (strcmp(s, "#-inf") == 0) return yay_float(-INFINITY);
            break;
    }
    return yay_string(s);
}

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
            yay_object_set(obj, key[0] == '!' ? key + 1 : key, json_value(p));
            free(key);
            json_space(p);
            if (**p == ',') (*p)++;
            json_space(p);
        }
        (*p)++;
        if (obj->data.object.length == 2 &&
            strcmp(obj->data.object.pairs[0].key, "#tag") == 0 &&
            strcmp(obj->data.object.pairs[1].key, "payload") == 0) {
            const char *tag = obj->data.object.pairs[0].value->data.string;
            yay_value_t *tagged = yay_tagged(tag[0] == '!' ? tag + 1 : tag,
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
        yay_value_t *v = ysonString(s);
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
            snprintf(path + strlen(path) - 4, 7, ".yson");
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

int main(void) {
    test_model();
    test_refused();
    test_parse();
    test_cases();
    printf("references: %d/%d checks passed\n", checks - failures, checks);
    return failures ? 1 : 0;
}
