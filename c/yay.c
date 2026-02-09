/*
 * YAY Parser - C Implementation
 */

#include "yay.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <stdarg.h>
#include <stdint.h>
#include <ctype.h>
#include <math.h>

/* ============================================================================
 * Internal Types
 * ============================================================================ */

/* Scan line from phase 1 */
typedef struct {
    char *line;         /* Content after indent and leader */
    int indent;         /* Number of leading spaces */
    char *leader;       /* "- " for list items, "" otherwise */
    int line_num;       /* Zero-based line number */
} scan_line_t;

/* Token types for phase 2 */
typedef enum {
    TOKEN_START,        /* Block start */
    TOKEN_STOP,         /* Block end */
    TOKEN_TEXT,         /* Text content */
    TOKEN_BREAK         /* Blank line */
} token_type_t;

/* Token from phase 2 */
typedef struct {
    token_type_t type;
    char *text;
    int indent;
    int line_num;
    int col;
} token_t;

/* Parse context */
typedef struct {
    const char *filename;
    const char *source;
    size_t source_len;
    
    /* Scan lines */
    scan_line_t *lines;
    size_t line_count;
    size_t line_capacity;
    
    /* Tokens */
    token_t *tokens;
    size_t token_count;
    size_t token_capacity;
    
    /* Error */
    yay_error_t *error;
} parse_ctx_t;

/* ============================================================================
 * Memory Helpers
 * ============================================================================ */

static char *str_dup(const char *s) {
    if (!s) return NULL;
    size_t len = strlen(s);
    char *copy = malloc(len + 1);
    if (copy) {
        memcpy(copy, s, len + 1);
    }
    return copy;
}

static char *str_dup_len(const char *s, size_t len) {
    if (!s) return NULL;
    char *copy = malloc(len + 1);
    if (copy) {
        memcpy(copy, s, len);
        copy[len] = '\0';
    }
    return copy;
}

/* ============================================================================
 * Value Constructors
 * ============================================================================ */

yay_value_t *yay_null(void) {
    yay_value_t *v = calloc(1, sizeof(yay_value_t));
    if (v) v->type = YAY_NULL;
    return v;
}

yay_value_t *yay_bool(bool value) {
    yay_value_t *v = calloc(1, sizeof(yay_value_t));
    if (v) {
        v->type = YAY_BOOL;
        v->data.boolean = value;
    }
    return v;
}

yay_value_t *yay_int_from_str(const char *digits, bool negative) {
    yay_value_t *v = calloc(1, sizeof(yay_value_t));
    if (v) {
        v->type = YAY_INT;
        v->data.bigint.digits = str_dup(digits);
        v->data.bigint.negative = negative;
    }
    return v;
}

yay_value_t *yay_int(int64_t value) {
    char buf[32];
    bool negative = value < 0;
    if (negative) value = -value;
    snprintf(buf, sizeof(buf), "%lld", (long long)value);
    return yay_int_from_str(buf, negative);
}

yay_value_t *yay_float(double value) {
    yay_value_t *v = calloc(1, sizeof(yay_value_t));
    if (v) {
        v->type = YAY_FLOAT;
        v->data.number = value;
    }
    return v;
}

yay_value_t *yay_string(const char *str) {
    yay_value_t *v = calloc(1, sizeof(yay_value_t));
    if (v) {
        v->type = YAY_STRING;
        v->data.string = str_dup(str);
    }
    return v;
}

yay_value_t *yay_string_len(const char *str, size_t len) {
    yay_value_t *v = calloc(1, sizeof(yay_value_t));
    if (v) {
        v->type = YAY_STRING;
        v->data.string = str_dup_len(str, len);
    }
    return v;
}

yay_value_t *yay_bytes(const uint8_t *data, size_t length) {
    yay_value_t *v = calloc(1, sizeof(yay_value_t));
    if (v) {
        v->type = YAY_BYTES;
        v->data.bytes.length = length;
        if (length > 0) {
            v->data.bytes.data = malloc(length);
            if (v->data.bytes.data) {
                memcpy(v->data.bytes.data, data, length);
            }
        }
    }
    return v;
}

static int hex_digit(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

/* Check if character is an uppercase hex digit */
static int is_uppercase_hex(char c) {
    return c >= 'A' && c <= 'F';
}

yay_value_t *yay_bytes_from_hex(const char *hex) {
    size_t len = strlen(hex);
    size_t byte_len = len / 2;
    uint8_t *data = malloc(byte_len);
    if (!data) return yay_bytes(NULL, 0);
    
    for (size_t i = 0; i < byte_len; i++) {
        int hi = hex_digit(hex[i * 2]);
        int lo = hex_digit(hex[i * 2 + 1]);
        data[i] = (hi << 4) | lo;
    }
    
    yay_value_t *v = yay_bytes(data, byte_len);
    free(data);
    return v;
}

yay_value_t *yay_array(void) {
    yay_value_t *v = calloc(1, sizeof(yay_value_t));
    if (v) {
        v->type = YAY_ARRAY;
        v->data.array.capacity = 8;
        v->data.array.items = calloc(8, sizeof(yay_value_t*));
    }
    return v;
}

yay_value_t *yay_object(void) {
    yay_value_t *v = calloc(1, sizeof(yay_value_t));
    if (v) {
        v->type = YAY_OBJECT;
        v->data.object.capacity = 8;
        v->data.object.pairs = calloc(8, sizeof(yay_pair_t));
    }
    return v;
}

yay_value_t *yay_array_push(yay_value_t *array, yay_value_t *item) {
    if (!array || array->type != YAY_ARRAY) return array;
    
    if (array->data.array.length >= array->data.array.capacity) {
        size_t new_cap = array->data.array.capacity * 2;
        yay_value_t **new_items = realloc(array->data.array.items, 
                                          new_cap * sizeof(yay_value_t*));
        if (!new_items) return array;
        array->data.array.items = new_items;
        array->data.array.capacity = new_cap;
    }
    
    array->data.array.items[array->data.array.length++] = item;
    return array;
}

yay_value_t *yay_object_set(yay_value_t *object, const char *key, yay_value_t *value) {
    if (!object || object->type != YAY_OBJECT) return object;
    
    /* Check for existing key */
    for (size_t i = 0; i < object->data.object.length; i++) {
        if (strcmp(object->data.object.pairs[i].key, key) == 0) {
            yay_free(object->data.object.pairs[i].value);
            object->data.object.pairs[i].value = value;
            return object;
        }
    }
    
    if (object->data.object.length >= object->data.object.capacity) {
        size_t new_cap = object->data.object.capacity * 2;
        yay_pair_t *new_pairs = realloc(object->data.object.pairs,
                                        new_cap * sizeof(yay_pair_t));
        if (!new_pairs) return object;
        object->data.object.pairs = new_pairs;
        object->data.object.capacity = new_cap;
    }
    
    size_t idx = object->data.object.length++;
    object->data.object.pairs[idx].key = str_dup(key);
    object->data.object.pairs[idx].value = value;
    return object;
}

yay_value_t *yay_array_of(yay_value_t **items, size_t count) {
    yay_value_t *arr = yay_array();
    for (size_t i = 0; i < count; i++) {
        yay_array_push(arr, items[i]);
    }
    return arr;
}

yay_value_t *yay_object_of(void **kvs, size_t count) {
    yay_value_t *obj = yay_object();
    for (size_t i = 0; i + 1 < count; i += 2) {
        yay_object_set(obj, (const char *)kvs[i], (yay_value_t *)kvs[i + 1]);
    }
    return obj;
}

/* ============================================================================
 * Value Destructor
 * ============================================================================ */

void yay_free(yay_value_t *value) {
    if (!value) return;
    
    switch (value->type) {
        case YAY_INT:
            free(value->data.bigint.digits);
            break;
        case YAY_STRING:
            free(value->data.string);
            break;
        case YAY_BYTES:
            free(value->data.bytes.data);
            break;
        case YAY_ARRAY:
            for (size_t i = 0; i < value->data.array.length; i++) {
                yay_free(value->data.array.items[i]);
            }
            free(value->data.array.items);
            break;
        case YAY_OBJECT:
            for (size_t i = 0; i < value->data.object.length; i++) {
                free(value->data.object.pairs[i].key);
                yay_free(value->data.object.pairs[i].value);
            }
            free(value->data.object.pairs);
            break;
        default:
            break;
    }
    
    free(value);
}

void yay_error_free(yay_error_t *error) {
    if (!error) return;
    free(error->message);
    free(error);
}

void yay_result_free(yay_result_t *result) {
    yay_free(result->value);
    yay_error_free(result->error);
}

/* ============================================================================
 * Value Comparison
 * ============================================================================ */

bool yay_equal(const yay_value_t *a, const yay_value_t *b) {
    if (a == b) return true;
    if (!a || !b) return false;
    if (a->type != b->type) return false;
    
    switch (a->type) {
        case YAY_NULL:
            return true;
        case YAY_BOOL:
            return a->data.boolean == b->data.boolean;
        case YAY_INT:
            return a->data.bigint.negative == b->data.bigint.negative &&
                   strcmp(a->data.bigint.digits, b->data.bigint.digits) == 0;
        case YAY_FLOAT:
            /* Handle NaN */
            if (isnan(a->data.number) && isnan(b->data.number)) return true;
            return a->data.number == b->data.number;
        case YAY_STRING:
            return strcmp(a->data.string, b->data.string) == 0;
        case YAY_BYTES:
            if (a->data.bytes.length != b->data.bytes.length) return false;
            return memcmp(a->data.bytes.data, b->data.bytes.data, 
                         a->data.bytes.length) == 0;
        case YAY_ARRAY:
            if (a->data.array.length != b->data.array.length) return false;
            for (size_t i = 0; i < a->data.array.length; i++) {
                if (!yay_equal(a->data.array.items[i], b->data.array.items[i])) {
                    return false;
                }
            }
            return true;
        case YAY_OBJECT:
            if (a->data.object.length != b->data.object.length) return false;
            for (size_t i = 0; i < a->data.object.length; i++) {
                /* Find matching key in b */
                bool found = false;
                for (size_t j = 0; j < b->data.object.length; j++) {
                    if (strcmp(a->data.object.pairs[i].key, 
                              b->data.object.pairs[j].key) == 0) {
                        if (!yay_equal(a->data.object.pairs[i].value,
                                      b->data.object.pairs[j].value)) {
                            return false;
                        }
                        found = true;
                        break;
                    }
                }
                if (!found) return false;
            }
            return true;
    }
    return false;
}

/* ============================================================================
 * Value to String (for debugging)
 * ============================================================================ */

static void append_str(char **buf, size_t *len, size_t *cap, const char *s) {
    size_t slen = strlen(s);
    while (*len + slen + 1 > *cap) {
        *cap *= 2;
        *buf = realloc(*buf, *cap);
    }
    memcpy(*buf + *len, s, slen);
    *len += slen;
    (*buf)[*len] = '\0';
}

static void value_to_string_impl(const yay_value_t *v, char **buf, 
                                  size_t *len, size_t *cap) {
    char tmp[64];
    
    if (!v) {
        append_str(buf, len, cap, "null");
        return;
    }
    
    switch (v->type) {
        case YAY_NULL:
            append_str(buf, len, cap, "null");
            break;
        case YAY_BOOL:
            append_str(buf, len, cap, v->data.boolean ? "true" : "false");
            break;
        case YAY_INT:
            if (v->data.bigint.negative) {
                append_str(buf, len, cap, "-");
            }
            append_str(buf, len, cap, v->data.bigint.digits);
            append_str(buf, len, cap, "n");
            break;
        case YAY_FLOAT:
            if (isnan(v->data.number)) {
                append_str(buf, len, cap, "NaN");
            } else if (isinf(v->data.number)) {
                append_str(buf, len, cap, v->data.number > 0 ? "Infinity" : "-Infinity");
            } else {
                snprintf(tmp, sizeof(tmp), "%g", v->data.number);
                append_str(buf, len, cap, tmp);
            }
            break;
        case YAY_STRING:
            append_str(buf, len, cap, "\"");
            append_str(buf, len, cap, v->data.string);
            append_str(buf, len, cap, "\"");
            break;
        case YAY_BYTES:
            append_str(buf, len, cap, "<");
            for (size_t i = 0; i < v->data.bytes.length; i++) {
                snprintf(tmp, sizeof(tmp), "%02x", v->data.bytes.data[i]);
                append_str(buf, len, cap, tmp);
            }
            append_str(buf, len, cap, ">");
            break;
        case YAY_ARRAY:
            append_str(buf, len, cap, "[");
            for (size_t i = 0; i < v->data.array.length; i++) {
                if (i > 0) append_str(buf, len, cap, ", ");
                value_to_string_impl(v->data.array.items[i], buf, len, cap);
            }
            append_str(buf, len, cap, "]");
            break;
        case YAY_OBJECT:
            append_str(buf, len, cap, "{");
            for (size_t i = 0; i < v->data.object.length; i++) {
                if (i > 0) append_str(buf, len, cap, ", ");
                append_str(buf, len, cap, v->data.object.pairs[i].key);
                append_str(buf, len, cap, ": ");
                value_to_string_impl(v->data.object.pairs[i].value, buf, len, cap);
            }
            append_str(buf, len, cap, "}");
            break;
    }
}

char *yay_to_string(const yay_value_t *value) {
    size_t cap = 256;
    size_t len = 0;
    char *buf = malloc(cap);
    if (!buf) return NULL;
    buf[0] = '\0';
    
    value_to_string_impl(value, &buf, &len, &cap);
    return buf;
}

/* ============================================================================
 * Error Helpers
 * ============================================================================ */

static yay_error_t *make_error(parse_ctx_t *ctx, int line, int col, 
                               const char *fmt, ...) {
    yay_error_t *err = calloc(1, sizeof(yay_error_t));
    if (!err) return NULL;
    
    char msg[512];
    va_list args;
    va_start(args, fmt);
    vsnprintf(msg, sizeof(msg), fmt, args);
    va_end(args);
    
    size_t msg_len = strlen(msg);
    if (ctx->filename) {
        size_t total = msg_len + strlen(ctx->filename) + 32;
        err->message = malloc(total);
        snprintf(err->message, total, "%s at %d:%d of <%s>", 
                 msg, line + 1, col + 1, ctx->filename);
    } else {
        err->message = str_dup(msg);
    }
    
    err->line = line + 1;
    err->column = col + 1;
    
    return err;
}

/* ============================================================================
 * Phase 1: Scanner
 * ============================================================================ */

static void add_scan_line(parse_ctx_t *ctx, const char *line, int indent,
                          const char *leader, int line_num) {
    if (ctx->line_count >= ctx->line_capacity) {
        ctx->line_capacity *= 2;
        ctx->lines = realloc(ctx->lines, ctx->line_capacity * sizeof(scan_line_t));
    }
    
    scan_line_t *sl = &ctx->lines[ctx->line_count++];
    sl->line = str_dup(line);
    sl->indent = indent;
    sl->leader = str_dup(leader);
    sl->line_num = line_num;
}

static bool scan(parse_ctx_t *ctx) {
    const char *src = ctx->source;
    size_t len = ctx->source_len;
    
    /* Check for BOM */
    if (len >= 3 && (unsigned char)src[0] == 0xEF && 
        (unsigned char)src[1] == 0xBB && (unsigned char)src[2] == 0xBF) {
        ctx->error = make_error(ctx, 0, 0, "Illegal BOM");
        return false;
    }
    
    /* Check for forbidden code points */
    {
        int line = 0, col = 0;
        size_t i = 0;
        while (i < len) {
            unsigned char c = src[i];
            uint32_t cp;
            size_t seq_len;
            /* Decode UTF-8 */
            if (c < 0x80) {
                cp = c;
                seq_len = 1;
            } else if ((c & 0xE0) == 0xC0 && i + 1 < len) {
                cp = ((uint32_t)(c & 0x1F) << 6) | (src[i+1] & 0x3F);
                seq_len = 2;
            } else if ((c & 0xF0) == 0xE0 && i + 2 < len) {
                cp = ((uint32_t)(c & 0x0F) << 12) | ((uint32_t)(src[i+1] & 0x3F) << 6) | (src[i+2] & 0x3F);
                seq_len = 3;
            } else if ((c & 0xF8) == 0xF0 && i + 3 < len) {
                cp = ((uint32_t)(c & 0x07) << 18) | ((uint32_t)(src[i+1] & 0x3F) << 12) | ((uint32_t)(src[i+2] & 0x3F) << 6) | (src[i+3] & 0x3F);
                seq_len = 4;
            } else {
                cp = c;
                seq_len = 1;
            }
            /* Check if allowed */
            int allowed = (cp == 0x000A)
                || (0x0020 <= cp && cp <= 0x007E)
                || (0x00A0 <= cp && cp <= 0xD7FF)
                || (0xE000 <= cp && cp <= 0xFFFD && !(0xFDD0 <= cp && cp <= 0xFDEF))
                || (0x10000 <= cp && cp <= 0x10FFFF && (cp & 0xFFFF) < 0xFFFE);
            if (!allowed) {
                if (cp == 0x09) {
                    ctx->error = make_error(ctx, line, col, "Tab not allowed (use spaces)");
                    return false;
                }
                if (cp >= 0xD800 && cp <= 0xDFFF) {
                    ctx->error = make_error(ctx, line, col, "Illegal surrogate");
                    return false;
                }
                ctx->error = make_error(ctx, line, col, "Forbidden code point U+%04X", cp);
                return false;
            }
            if (cp == 0x0A) {
                line++;
                col = 0;
            } else {
                col++;
            }
            i += seq_len;
        }
    }
    
    /* Process lines */
    const char *line_start = src;
    int line_num = 0;
    
    while (line_start <= src + len) {
        /* Find end of line */
        const char *line_end = line_start;
        while (line_end < src + len && *line_end != '\n') {
            line_end++;
        }
        
        size_t line_len = line_end - line_start;
        
        /* Check for trailing space */
        if (line_len > 0 && line_start[line_len - 1] == ' ') {
            ctx->error = make_error(ctx, line_num, (int)line_len - 1, 
                                    "Unexpected trailing space");
            return false;
        }
        
        /* Count indent */
        int indent = 0;
        while (indent < (int)line_len && line_start[indent] == ' ') {
            indent++;
        }
        
        const char *rest = line_start + indent;
        size_t rest_len = line_len - indent;
        
        /* Skip top-level comments */
        if (rest_len > 0 && rest[0] == '#' && indent == 0) {
            line_start = line_end + 1;
            line_num++;
            continue;
        }
        
        /* Extract leader and content */
        const char *leader = "";
        const char *content = rest;
        size_t content_len = rest_len;
        
        if (rest_len >= 2 && rest[0] == '-' && rest[1] == ' ') {
            leader = "- ";
            content = rest + 2;
            content_len = rest_len - 2;
        } else if (rest_len == 1 && rest[0] == '-') {
            leader = "- ";
            content = "";
            content_len = 0;
        } else if (rest_len >= 2 && rest[0] == '-' && rest[1] != ' ' && 
                   rest[1] != '.' && !(rest[1] >= '0' && rest[1] <= '9') &&
                   strncmp(rest, "-infinity", 9) != 0) {
            /* Compact list syntax (-value without space) is not allowed */
            ctx->error = make_error(ctx, line_num, indent + 1, 
                                    "Expected space after \"-\"");
            return false;
        } else if (rest_len >= 1 && rest[0] == '*' && 
                   (rest_len == 1 || rest[1] == ' ')) {
            ctx->error = make_error(ctx, line_num, indent, 
                                    "Unexpected character \"*\"");
            return false;
        }
        
        /* Add the line */
        char *content_str = str_dup_len(content, content_len);
        add_scan_line(ctx, content_str, indent, leader, line_num);
        free(content_str);
        
        if (line_end >= src + len) break;
        line_start = line_end + 1;
        line_num++;
    }
    
    return true;
}

/* ============================================================================
 * Phase 2: Outline Lexer
 * ============================================================================ */

static void add_token(parse_ctx_t *ctx, token_type_t type, const char *text,
                      int indent, int line_num, int col) {
    if (ctx->token_count >= ctx->token_capacity) {
        ctx->token_capacity *= 2;
        ctx->tokens = realloc(ctx->tokens, ctx->token_capacity * sizeof(token_t));
    }
    
    token_t *t = &ctx->tokens[ctx->token_count++];
    t->type = type;
    t->text = str_dup(text);
    t->indent = indent;
    t->line_num = line_num;
    t->col = col;
}

static void outline_lex(parse_ctx_t *ctx) {
    int stack[256];
    int stack_top = 0;
    stack[0] = 0;
    int top = 0;
    bool broken = false;
    
    for (size_t i = 0; i < ctx->line_count; i++) {
        scan_line_t *sl = &ctx->lines[i];
        
        /* Emit stops for dedent */
        while (sl->indent < top) {
            add_token(ctx, TOKEN_STOP, "", 0, 0, 0);
            stack_top--;
            top = stack[stack_top];
        }
        
        /* Emit start for list items */
        if (strlen(sl->leader) > 0) {
            if (sl->indent > top) {
                add_token(ctx, TOKEN_START, sl->leader, sl->indent, 
                         sl->line_num, sl->indent);
                stack[++stack_top] = sl->indent;
                top = sl->indent;
                broken = false;
            } else if (sl->indent == top) {
                add_token(ctx, TOKEN_STOP, "", 0, 0, 0);
                add_token(ctx, TOKEN_START, sl->leader, sl->indent,
                         sl->line_num, sl->indent);
                broken = false;
            }
        }
        
        /* Emit text or break */
        if (strlen(sl->line) > 0) {
            add_token(ctx, TOKEN_TEXT, sl->line, sl->indent, 
                     sl->line_num, sl->indent);
            broken = false;
        } else if (!broken) {
            add_token(ctx, TOKEN_BREAK, "", sl->line_num, sl->indent, 0);
            broken = true;
        }
    }
    
    /* Close remaining blocks */
    while (stack_top > 0) {
        add_token(ctx, TOKEN_STOP, "", 0, 0, 0);
        stack_top--;
    }
}

/* ============================================================================
 * Phase 3: Value Parser - Forward Declarations
 * ============================================================================ */

static yay_value_t *parse_value(parse_ctx_t *ctx, size_t *idx);
static yay_value_t *parse_multiline_array(parse_ctx_t *ctx, size_t *idx);
static yay_value_t *parse_multiline_array_impl(parse_ctx_t *ctx, size_t *idx, int min_indent);
static yay_value_t *parse_scalar(parse_ctx_t *ctx, const char *s, 
                                  int line_num, int col);
static yay_value_t *parse_concatenated_strings(parse_ctx_t *ctx, size_t *idx, int base_indent);

/* ============================================================================
 * Helper Functions
 * ============================================================================ */

static size_t skip_breaks_and_stops(parse_ctx_t *ctx, size_t i) {
    while (i < ctx->token_count && 
           (ctx->tokens[i].type == TOKEN_STOP || 
            ctx->tokens[i].type == TOKEN_BREAK)) {
        i++;
    }
    return i;
}

static size_t skip_breaks(parse_ctx_t *ctx, size_t i) {
    while (i < ctx->token_count && ctx->tokens[i].type == TOKEN_BREAK) {
        i++;
    }
    return i;
}

static size_t skip_stops(parse_ctx_t *ctx, size_t i) {
    while (i < ctx->token_count && ctx->tokens[i].type == TOKEN_STOP) {
        i++;
    }
    return i;
}

/* Find colon outside quotes */
static int find_colon_outside_quotes(const char *s) {
    bool in_double = false;
    bool in_single = false;
    bool escape = false;
    
    for (int i = 0; s[i]; i++) {
        char c = s[i];
        
        if (escape) {
            escape = false;
            continue;
        }
        
        if (c == '\\' && (in_double || in_single)) {
            escape = true;
            continue;
        }
        
        if (c == '"' && !in_single) {
            in_double = !in_double;
        } else if (c == '\'' && !in_double) {
            in_single = !in_single;
        } else if (c == ':' && !in_double && !in_single) {
            return i;
        }
    }
    
    return -1;
}

/* Parse key name (handles quoted keys) */
static char *parse_key_name(const char *s) {
    /* Skip leading whitespace */
    while (*s == ' ') s++;
    
    size_t len = strlen(s);
    
    /* Double-quoted key */
    if (len >= 2 && s[0] == '"' && s[len-1] == '"') {
        return str_dup_len(s + 1, len - 2);
    }
    
    /* Single-quoted key */
    if (len >= 2 && s[0] == '\'' && s[len-1] == '\'') {
        return str_dup_len(s + 1, len - 2);
    }
    
    /* Trim trailing whitespace */
    while (len > 0 && s[len-1] == ' ') len--;
    
    return str_dup_len(s, len);
}

/* ============================================================================
 * Number Parsing
 * ============================================================================ */

static bool is_integer_str(const char *s) {
    if (*s == '-') s++;
    if (!*s) return false;
    while (*s) {
        if (*s != ' ' && (*s < '0' || *s > '9')) return false;
        s++;
    }
    return true;
}

static bool is_float_str(const char *s) {
    bool has_dot = false;
    bool has_exponent = false;
    bool has_digit = false;
    
    if (*s == '-') s++;
    
    while (*s) {
        if (*s == '.') {
            if (has_dot || has_exponent) return false;
            has_dot = true;
        } else if (*s == 'e' || *s == 'E') {
            if (has_exponent || !has_digit) return false;
            has_exponent = true;
            s++;
            /* Allow optional +/- after exponent */
            if (*s == '+' || *s == '-') s++;
            continue;
        } else if (*s >= '0' && *s <= '9') {
            has_digit = true;
        } else if (*s != ' ') {
            return false;
        }
        s++;
    }
    
    return (has_dot || has_exponent) && has_digit;
}

static bool has_exponent(const char *s) {
    while (*s) {
        if (*s == 'e' || *s == 'E') return true;
        s++;
    }
    return false;
}

static yay_value_t *parse_number_with_validation(parse_ctx_t *ctx, const char *s,
                                                  int line_num, int col) {
    /* Check for uppercase E in exponent */
    const char *e_pos = strchr(s, 'E');
    if (e_pos) {
        ctx->error = make_error(ctx, line_num, col + (int)(e_pos - s),
                               "Uppercase exponent (use lowercase 'e')");
        return NULL;
    }
    
    /* Validate spaces are not around decimal point */
    const char *dot = strchr(s, '.');
    if (dot) {
        /* Check for space before dot */
        if (dot > s && *(dot - 1) == ' ') {
            ctx->error = make_error(ctx, line_num, col + (int)(dot - s - 1),
                                   "Unexpected space in number");
            return NULL;
        }
        /* Check for space after dot */
        if (*(dot + 1) == ' ') {
            ctx->error = make_error(ctx, line_num, col + (int)(dot - s + 1),
                                   "Unexpected space in number");
            return NULL;
        }
    }
    
    /* Remove spaces */
    char *compact = malloc(strlen(s) + 1);
    char *p = compact;
    for (const char *q = s; *q; q++) {
        if (*q != ' ') *p++ = *q;
    }
    *p = '\0';
    
    /* Check for float (has decimal point or exponent) */
    if (strchr(compact, '.') || has_exponent(compact)) {
        double val = atof(compact);
        free(compact);
        return yay_float(val);
    }
    
    /* Integer */
    bool negative = compact[0] == '-';
    char *digits = negative ? compact + 1 : compact;
    
    yay_value_t *v = yay_int_from_str(digits, negative);
    free(compact);
    return v;
}

/* ============================================================================
 * String Parsing
 * ============================================================================ */

static yay_value_t *parse_double_quoted_string(parse_ctx_t *ctx, const char *s,
                                                int line_num, int col) {
    size_t len = strlen(s);
    if (len < 2 || s[0] != '"' || s[len-1] != '"') {
        return yay_string(s);
    }
    
    char *out = malloc(len);
    size_t out_len = 0;
    
    for (size_t i = 1; i < len - 1; i++) {
        char c = s[i];
        
        if (c == '\\') {
            if (i + 1 >= len - 1) {
                ctx->error = make_error(ctx, line_num, col + i + 1,
                                       "Bad escaped character");
                free(out);
                return NULL;
            }
            
            char esc = s[++i];
            switch (esc) {
                case '"': out[out_len++] = '"'; break;
                case '\\': out[out_len++] = '\\'; break;
                case '/': out[out_len++] = '/'; break;
                case 'b': out[out_len++] = '\b'; break;
                case 'f': out[out_len++] = '\f'; break;
                case 'n': out[out_len++] = '\n'; break;
                case 'r': out[out_len++] = '\r'; break;
                case 't': out[out_len++] = '\t'; break;
                case 'u': {
                    /* Expect \u{XXXXXX} format */
                    /* Old-style \uXXXX is not supported - report at 'u' column */
                    if (i + 1 >= len - 1 || s[i+1] != '{') {
                        ctx->error = make_error(ctx, line_num, col + i,
                                               "Bad escaped character");
                        free(out);
                        return NULL;
                    }
                    
                    /* Column of '{' for subsequent errors */
                    int brace_col = col + i + 1;
                    
                    /* Find closing brace */
                    size_t brace_end = i + 2;
                    while (brace_end < len - 1 && s[brace_end] != '}') {
                        brace_end++;
                    }
                    
                    if (brace_end >= len - 1 || s[brace_end] != '}') {
                        ctx->error = make_error(ctx, line_num, brace_col,
                                               "Bad Unicode escape");
                        free(out);
                        return NULL;
                    }
                    
                    size_t hex_start = i + 2;
                    size_t hex_len_u = brace_end - hex_start;
                    
                    if (hex_len_u == 0 || hex_len_u > 6) {
                        ctx->error = make_error(ctx, line_num, brace_col,
                                               "Bad Unicode escape");
                        free(out);
                        return NULL;
                    }
                    
                    /* Validate hex digits */
                    for (size_t j = hex_start; j < brace_end; j++) {
                        if (!isxdigit((unsigned char)s[j])) {
                            ctx->error = make_error(ctx, line_num, brace_col,
                                                   "Bad Unicode escape");
                            free(out);
                            return NULL;
                        }
                    }
                    
                    char hex[7];
                    memcpy(hex, s + hex_start, hex_len_u);
                    hex[hex_len_u] = '\0';
                    
                    unsigned int code;
                    if (sscanf(hex, "%x", &code) != 1) {
                        ctx->error = make_error(ctx, line_num, brace_col,
                                               "Bad Unicode escape");
                        free(out);
                        return NULL;
                    }
                    
                    /* Check for surrogates */
                    if (code >= 0xD800 && code <= 0xDFFF) {
                        ctx->error = make_error(ctx, line_num, brace_col,
                                               "Illegal surrogate");
                        free(out);
                        return NULL;
                    }
                    
                    /* Check for out of range */
                    if (code > 0x10FFFF) {
                        ctx->error = make_error(ctx, line_num, brace_col,
                                               "Unicode code point out of range");
                        free(out);
                        return NULL;
                    }
                    
                    /* Encode as UTF-8 */
                    if (code < 0x80) {
                        out[out_len++] = code;
                    } else if (code < 0x800) {
                        out[out_len++] = 0xC0 | (code >> 6);
                        out[out_len++] = 0x80 | (code & 0x3F);
                    } else if (code < 0x10000) {
                        out[out_len++] = 0xE0 | (code >> 12);
                        out[out_len++] = 0x80 | ((code >> 6) & 0x3F);
                        out[out_len++] = 0x80 | (code & 0x3F);
                    } else {
                        out[out_len++] = 0xF0 | (code >> 18);
                        out[out_len++] = 0x80 | ((code >> 12) & 0x3F);
                        out[out_len++] = 0x80 | ((code >> 6) & 0x3F);
                        out[out_len++] = 0x80 | (code & 0x3F);
                    }
                    
                    i = brace_end;
                    break;
                }
                default:
                    ctx->error = make_error(ctx, line_num, col + i,
                                           "Bad escaped character");
                    free(out);
                    return NULL;
            }
        } else if ((unsigned char)c < 0x20) {
            ctx->error = make_error(ctx, line_num, col + i,
                                   "Bad character in string");
            free(out);
            return NULL;
        } else {
            out[out_len++] = c;
        }
    }
    
    out[out_len] = '\0';
    yay_value_t *v = yay_string(out);
    free(out);
    return v;
}

static yay_value_t *parse_single_quoted_string(const char *s) {
    size_t len = strlen(s);
    if (len < 2 || s[0] != '\'' || s[len-1] != '\'') {
        return yay_string(s);
    }
    return yay_string_len(s + 1, len - 2);
}

/* ============================================================================
 * Concatenated String Parsing
 * ============================================================================ */

/**
 * Parse concatenated quoted strings (multiple quoted strings on consecutive lines).
 * Returns NULL if there's only one string (single string on new line is invalid).
 */
static yay_value_t *parse_concatenated_strings(parse_ctx_t *ctx, size_t *idx, int base_indent) {
    /* Collect all parts */
    size_t parts_cap = 8;
    size_t parts_count = 0;
    char **parts = malloc(parts_cap * sizeof(char *));
    size_t *part_lens = malloc(parts_cap * sizeof(size_t));
    
    while (*idx < ctx->token_count) {
        token_t *t = &ctx->tokens[*idx];
        
        if (t->type == TOKEN_BREAK || t->type == TOKEN_STOP) {
            (*idx)++;
            continue;
        }
        
        if (t->type != TOKEN_TEXT || t->indent < base_indent) {
            break;
        }
        
        /* Trim leading spaces */
        const char *trimmed = t->text;
        while (*trimmed == ' ') trimmed++;
        size_t trimmed_len = strlen(trimmed);
        
        /* Check if this line is a quoted string */
        bool is_double_quoted = trimmed_len >= 2 && trimmed[0] == '"' && trimmed[trimmed_len-1] == '"';
        bool is_single_quoted = trimmed_len >= 2 && trimmed[0] == '\'' && trimmed[trimmed_len-1] == '\'';
        
        if (!is_double_quoted && !is_single_quoted) {
            break;
        }
        
        /* Parse the quoted string */
        yay_value_t *parsed;
        if (is_double_quoted) {
            parsed = parse_double_quoted_string(ctx, trimmed, t->line_num, t->col);
        } else {
            parsed = parse_single_quoted_string(trimmed);
        }
        
        if (parsed == NULL) {
            /* Error already set */
            for (size_t i = 0; i < parts_count; i++) {
                free(parts[i]);
            }
            free(parts);
            free(part_lens);
            return NULL;
        }
        
        /* Store the parsed string */
        if (parts_count >= parts_cap) {
            parts_cap *= 2;
            parts = realloc(parts, parts_cap * sizeof(char *));
            part_lens = realloc(part_lens, parts_cap * sizeof(size_t));
        }
        parts[parts_count] = str_dup(parsed->data.string);
        part_lens[parts_count] = strlen(parsed->data.string);
        parts_count++;
        
        yay_free(parsed);
        (*idx)++;
    }
    
    /* Require at least 2 strings for concatenation */
    /* A single string on a new line is invalid (use inline syntax instead) */
    if (parts_count < 2) {
        for (size_t i = 0; i < parts_count; i++) {
            free(parts[i]);
        }
        free(parts);
        free(part_lens);
        return NULL;
    }
    
    /* Calculate total length */
    size_t total_len = 0;
    for (size_t i = 0; i < parts_count; i++) {
        total_len += part_lens[i];
    }
    
    /* Concatenate all parts */
    char *result = malloc(total_len + 1);
    size_t pos = 0;
    for (size_t i = 0; i < parts_count; i++) {
        memcpy(result + pos, parts[i], part_lens[i]);
        pos += part_lens[i];
        free(parts[i]);
    }
    result[total_len] = '\0';
    
    free(parts);
    free(part_lens);
    
    yay_value_t *val = yay_string_len(result, total_len);
    free(result);
    return val;
}

/* ============================================================================
 * Block String Parsing
 * ============================================================================ */

static yay_value_t *parse_block_string_impl(parse_ctx_t *ctx, size_t *idx, 
                                             const char *first_line,
                                             int base_indent) {
    size_t i = *idx + 1;
    bool is_property = (base_indent >= 0);
    
    /* Collect continuation lines */
    size_t line_cap = 16;
    size_t line_count = 0;
    char **lines = malloc(line_cap * sizeof(char*));
    int *indents = malloc(line_cap * sizeof(int));
    
    if (first_line && strlen(first_line) > 0) {
        lines[line_count] = str_dup(first_line);
        indents[line_count] = -1; /* Mark as first line */
        line_count++;
    }
    
    while (i < ctx->token_count && 
           (ctx->tokens[i].type == TOKEN_TEXT || 
            ctx->tokens[i].type == TOKEN_BREAK)) {
        /* For property context, stop at lines at or below base indent */
        if (is_property && ctx->tokens[i].type == TOKEN_TEXT &&
            ctx->tokens[i].indent <= base_indent) {
            break;
        }
        
        if (line_count >= line_cap) {
            line_cap *= 2;
            lines = realloc(lines, line_cap * sizeof(char*));
            indents = realloc(indents, line_cap * sizeof(int));
        }
        
        if (ctx->tokens[i].type == TOKEN_BREAK) {
            lines[line_count] = str_dup("");
            indents[line_count] = -2; /* Mark as break */
        } else {
            lines[line_count] = str_dup(ctx->tokens[i].text);
            indents[line_count] = ctx->tokens[i].indent;
        }
        line_count++;
        i++;
    }
    
    *idx = i;
    
    /* Find minimum indent */
    int min_indent = INT32_MAX;
    for (size_t j = 0; j < line_count; j++) {
        if (indents[j] >= 0 && indents[j] < min_indent) {
            min_indent = indents[j];
        }
    }
    if (min_indent == INT32_MAX) min_indent = 0;
    
    /* Build result */
    size_t result_cap = 256;
    size_t result_len = 0;
    char *result = malloc(result_cap);
    result[0] = '\0';
    
    /* Leading newline if first_line was empty AND not a property */
    bool leading_newline = (!first_line || strlen(first_line) == 0) && 
                           line_count > 0 && !is_property;
    
    /* Skip leading empty lines */
    size_t start = 0;
    if (first_line && strlen(first_line) > 0) {
        start = 0;
    } else {
        while (start < line_count && strlen(lines[start]) == 0) start++;
    }
    
    /* Skip trailing empty lines */
    size_t end = line_count;
    while (end > start && strlen(lines[end-1]) == 0) end--;
    
    if (leading_newline && end > start) {
        result[result_len++] = '\n';
    }
    
    for (size_t j = start; j < end; j++) {
        if (j > start) {
            result[result_len++] = '\n';
        }
        
        /* Add extra indent */
        int extra = (indents[j] >= 0) ? indents[j] - min_indent : 0;
        while (extra-- > 0) {
            if (result_len + 1 >= result_cap) {
                result_cap *= 2;
                result = realloc(result, result_cap);
            }
            result[result_len++] = ' ';
        }
        
        /* Add line content */
        size_t line_len = strlen(lines[j]);
        while (result_len + line_len + 1 >= result_cap) {
            result_cap *= 2;
            result = realloc(result, result_cap);
        }
        memcpy(result + result_len, lines[j], line_len);
        result_len += line_len;
    }
    
    /* Trailing newline */
    if (end > start) {
        result[result_len++] = '\n';
    }
    result[result_len] = '\0';
    
    /* Check for empty block string */
    if (result_len == 0) {
        /* Cleanup */
        for (size_t j = 0; j < line_count; j++) {
            free(lines[j]);
        }
        free(lines);
        free(indents);
        free(result);
        ctx->error = calloc(1, sizeof(yay_error_t));
        ctx->error->message = str_dup("Empty block string not allowed (use \"\" or \"\\n\" explicitly)");
        return NULL;
    }
    
    /* Cleanup */
    for (size_t j = 0; j < line_count; j++) {
        free(lines[j]);
    }
    free(lines);
    free(indents);
    
    yay_value_t *v = yay_string(result);
    free(result);
    return v;
}

/* Wrapper for standalone block strings */
static yay_value_t *parse_block_string(parse_ctx_t *ctx, size_t *idx, 
                                        const char *first_line) {
    return parse_block_string_impl(ctx, idx, first_line, -1);
}

/* Wrapper for property block strings (with base indent) */
static yay_value_t *parse_property_block_string_indent(parse_ctx_t *ctx, size_t *idx, 
                                                        const char *first_line,
                                                        int base_indent) {
    return parse_block_string_impl(ctx, idx, first_line, base_indent);
}

/* ============================================================================
 * Block Bytes Parsing (> hex)
 * ============================================================================ */

/* Parse block bytes starting with > */
static yay_value_t *parse_block_bytes(parse_ctx_t *ctx, size_t *idx) {
    token_t *t = &ctx->tokens[*idx];
    const char *s = t->text;
    int base_indent = t->indent;
    
    /* Extract hex from first line (after >) */
    const char *hex_start = s + 1;
    int hex_col_offset = 1;
    if (*hex_start == ' ') {
        hex_start++;
        hex_col_offset = 2;
    }
    
    /* Strip comment */
    char *first_hex = str_dup(hex_start);
    char *comment = strchr(first_hex, '#');
    if (comment) *comment = '\0';
    
    /* Trim whitespace from first_hex */
    char *trimmed = first_hex;
    while (*trimmed == ' ') trimmed++;
    size_t trimmed_len = strlen(trimmed);
    while (trimmed_len > 0 && trimmed[trimmed_len - 1] == ' ') {
        trimmed[--trimmed_len] = '\0';
    }
    
    /* Check for empty leader (just ">" with no hex or comment) */
    if (*trimmed == '\0' && comment == NULL) {
        ctx->error = calloc(1, sizeof(yay_error_t));
        ctx->error->message = str_dup("Expected hex or comment in hex block");
        free(first_hex);
        return NULL;
    }
    
    size_t hex_cap = 256;
    size_t hex_len = 0;
    char *hex = malloc(hex_cap);
    
    /* Add hex from first line, checking for uppercase */
    for (char *p = first_hex; *p; p++) {
        if (*p != ' ') {
            if (is_uppercase_hex(*p)) {
                ctx->error = make_error(ctx, t->line_num, t->col + hex_col_offset + (int)(p - first_hex),
                                       "Uppercase hex digit (use lowercase)");
                free(first_hex);
                free(hex);
                return NULL;
            }
            if (hex_len >= hex_cap - 1) {
                hex_cap *= 2;
                hex = realloc(hex, hex_cap);
            }
            hex[hex_len++] = *p;
        }
    }
    free(first_hex);
    
    (*idx)++;
    
    /* Collect continuation lines */
    while (*idx < ctx->token_count && 
           ctx->tokens[*idx].type == TOKEN_TEXT &&
           ctx->tokens[*idx].indent > base_indent) {
        token_t *line_tok = &ctx->tokens[*idx];
        char *line = str_dup(line_tok->text);
        char *line_comment = strchr(line, '#');
        if (line_comment) *line_comment = '\0';
        
        for (char *p = line; *p; p++) {
            if (*p != ' ') {
                if (is_uppercase_hex(*p)) {
                    ctx->error = make_error(ctx, line_tok->line_num, line_tok->col + (int)(p - line),
                                           "Uppercase hex digit (use lowercase)");
                    free(line);
                    free(hex);
                    return NULL;
                }
                if (hex_len >= hex_cap - 1) {
                    hex_cap *= 2;
                    hex = realloc(hex, hex_cap);
                }
                hex[hex_len++] = *p;
            }
        }
        free(line);
        (*idx)++;
    }
    
    hex[hex_len] = '\0';
    
    if (hex_len % 2 != 0) {
        ctx->error = make_error(ctx, t->line_num, t->col,
                               "Odd number of hex digits in byte literal");
        free(hex);
        return NULL;
    }
    
    yay_value_t *v = yay_bytes_from_hex(hex);
    free(hex);
    return v;
}

/* Parse block bytes in property context (key: > hex) */
static yay_value_t *parse_property_block_bytes(parse_ctx_t *ctx, size_t *idx, 
                                                const char *v_part) {
    token_t *t = &ctx->tokens[*idx];
    int base_indent = t->indent;
    
    /* Extract hex from first line (after >) */
    const char *hex_start = v_part + 1;
    if (*hex_start == ' ') hex_start++;
    
    /* Strip comment */
    char *first_hex = str_dup(hex_start);
    char *comment = strchr(first_hex, '#');
    if (comment) *comment = '\0';
    
    size_t hex_cap = 256;
    size_t hex_len = 0;
    char *hex = malloc(hex_cap);
    
    /* Add hex from first line */
    for (char *p = first_hex; *p; p++) {
        if (*p != ' ') {
            if (hex_len >= hex_cap - 1) {
                hex_cap *= 2;
                hex = realloc(hex, hex_cap);
            }
            hex[hex_len++] = tolower(*p);
        }
    }
    free(first_hex);
    
    (*idx)++;
    
    /* Collect continuation lines */
    while (*idx < ctx->token_count && 
           ctx->tokens[*idx].type == TOKEN_TEXT &&
           ctx->tokens[*idx].indent > base_indent) {
        char *line = str_dup(ctx->tokens[*idx].text);
        char *line_comment = strchr(line, '#');
        if (line_comment) *line_comment = '\0';
        
        for (char *p = line; *p; p++) {
            if (*p != ' ') {
                if (hex_len >= hex_cap - 1) {
                    hex_cap *= 2;
                    hex = realloc(hex, hex_cap);
                }
                hex[hex_len++] = tolower(*p);
            }
        }
        free(line);
        (*idx)++;
    }
    
    hex[hex_len] = '\0';
    
    if (hex_len % 2 != 0) {
        ctx->error = make_error(ctx, t->line_num, t->col,
                               "Odd number of hex digits in byte literal");
        free(hex);
        return NULL;
    }
    
    yay_value_t *v = yay_bytes_from_hex(hex);
    free(hex);
    return v;
}

/* ============================================================================
 * Inline Syntax Validation
 * ============================================================================ */

/* Validate inline array/object syntax for whitespace rules.
 * Returns true if valid, false and sets error if invalid.
 * Rules:
 * - No space after [ or {
 * - No space before ] or }
 * - No space before ,
 * - Exactly one space after , (unless followed by ] or })
 * - No tabs
 * - Exactly one space after : in objects
 * - No space before : in objects
 */
static bool validate_inline_syntax(parse_ctx_t *ctx, const char *s, 
                                   int line_num, int col) {
    bool in_string = false;
    char string_char = 0;
    bool escape = false;
    int depth_bracket = 0;
    int depth_brace = 0;
    
    for (int i = 0; s[i]; i++) {
        char c = s[i];
        
        /* Handle escape sequences in strings */
        if (escape) {
            escape = false;
            continue;
        }
        
        /* Handle strings */
        if (in_string) {
            if (c == '\\') {
                escape = true;
            } else if (c == string_char) {
                in_string = false;
                string_char = 0;
            }
            continue;
        }
        
        if (c == '"' || c == '\'') {
            in_string = true;
            string_char = c;
            continue;
        }
        
        /* Check for newlines */
        if (c == '\n') {
            if (depth_bracket > 0) {
                ctx->error = make_error(ctx, line_num, col, 
                                       "Unexpected newline in inline array");
                return false;
            }
            if (depth_brace > 0) {
                ctx->error = make_error(ctx, line_num, col, 
                                       "Unexpected newline in inline object");
                return false;
            }
            continue;
        }
        
        /* Check [ */
        if (c == '[') {
            depth_bracket++;
            if (s[i+1] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i + 1, 
                                       "Unexpected space after \"[\"");
                return false;
            }
            continue;
        }
        
        /* Check ] */
        if (c == ']') {
            if (i > 0 && s[i-1] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i - 1, 
                                       "Unexpected space before \"]\"");
                return false;
            }
            depth_bracket--;
            continue;
        }
        
        /* Check { */
        if (c == '{') {
            depth_brace++;
            if (s[i+1] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i + 1, 
                                       "Unexpected space after \"{\"");
                return false;
            }
            continue;
        }
        
        /* Check } */
        if (c == '}') {
            if (i > 0 && s[i-1] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i - 1, 
                                       "Unexpected space before \"}\"");
                return false;
            }
            depth_brace--;
            continue;
        }
        
        /* Check < for byte arrays */
        if (c == '<') {
            if (s[i+1] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i + 1, 
                                       "Unexpected space after \"<\"");
                return false;
            }
            continue;
        }
        
        /* Check > */
        if (c == '>') {
            if (i > 0 && s[i-1] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i - 1, 
                                       "Unexpected space before \">\"");
                return false;
            }
            continue;
        }
        
        /* Check , */
        if (c == ',') {
            if (i > 0 && s[i-1] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i - 1, 
                                       "Unexpected space before \",\"");
                return false;
            }
            /* Check for space after comma */
            if (s[i+1] && s[i+1] != ' ' && s[i+1] != ']' && s[i+1] != '}') {
                /* Lookahead to see if next closing bracket has space before it */
                /* If so, don't report "Expected space after comma" - let the
                 * "Unexpected space before ]" error be reported instead */
                bool next_close_has_space = false;
                int la_depth = depth_bracket + depth_brace;
                bool la_in_string = false;
                char la_string_char = 0;
                bool la_escape = false;
                for (int j = i + 1; s[j]; j++) {
                    if (la_escape) {
                        la_escape = false;
                        continue;
                    }
                    if (la_in_string) {
                        if (s[j] == '\\') la_escape = true;
                        else if (s[j] == la_string_char) {
                            la_in_string = false;
                            la_string_char = 0;
                        }
                        continue;
                    }
                    if (s[j] == '"' || s[j] == '\'') {
                        la_in_string = true;
                        la_string_char = s[j];
                        continue;
                    }
                    if (s[j] == '[' || s[j] == '{') {
                        la_depth++;
                        continue;
                    }
                    if (s[j] == ']' || s[j] == '}') {
                        if (la_depth == depth_bracket + depth_brace) {
                            /* Found matching close at same depth */
                            next_close_has_space = (j > 0 && s[j-1] == ' ');
                            break;
                        }
                        la_depth--;
                        continue;
                    }
                    if (s[j] == ',' && la_depth == depth_bracket + depth_brace) {
                        /* Found another comma at same depth - stop lookahead */
                        break;
                    }
                }
                if (!next_close_has_space) {
                    ctx->error = make_error(ctx, line_num, col + i, 
                                           "Expected space after \",\"");
                    return false;
                }
            }
            /* Check for double space after comma */
            if (s[i+1] == ' ' && s[i+2] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i + 2, 
                                       "Unexpected space after \",\"");
                return false;
            }
            continue;
        }
        
        /* Check : in objects (only at depth > 0) */
        if (c == ':' && depth_brace > 0) {
            if (i > 0 && s[i-1] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i - 1, 
                                       "Unexpected space before \":\"");
                return false;
            }
            if (s[i+1] != ' ') {
                ctx->error = make_error(ctx, line_num, col + i, 
                                       "Expected space after \":\"");
                return false;
            }
            if (s[i+1] == ' ' && s[i+2] == ' ') {
                ctx->error = make_error(ctx, line_num, col + i + 2, 
                                       "Unexpected space after \":\"");
                return false;
            }
            continue;
        }
    }
    
    return true;
}

/* ============================================================================
 * Inline Array Parsing
 * ============================================================================ */

static yay_value_t *parse_inline_value(parse_ctx_t *ctx, const char *s, 
                                        size_t *consumed, int line_num, int col);

static int find_matching_bracket(const char *s) {
    int depth = 0;
    bool in_string = false;
    char string_char = 0;
    bool escape = false;
    
    for (int i = 0; s[i]; i++) {
        if (escape) {
            escape = false;
            continue;
        }
        if (s[i] == '\\' && in_string) {
            escape = true;
            continue;
        }
        if ((s[i] == '"' || s[i] == '\'') && (!in_string || s[i] == string_char)) {
            if (in_string) {
                in_string = false;
                string_char = 0;
            } else {
                in_string = true;
                string_char = s[i];
            }
            continue;
        }
        if (in_string) continue;
        
        if (s[i] == '[') depth++;
        else if (s[i] == ']') {
            depth--;
            if (depth == 0) return i;
        }
    }
    return -1;
}

static int find_matching_brace(const char *s) {
    int depth = 0;
    bool in_string = false;
    char string_char = 0;
    bool escape = false;
    
    for (int i = 0; s[i]; i++) {
        if (escape) {
            escape = false;
            continue;
        }
        if (s[i] == '\\' && in_string) {
            escape = true;
            continue;
        }
        if ((s[i] == '"' || s[i] == '\'') && (!in_string || s[i] == string_char)) {
            if (in_string) {
                in_string = false;
                string_char = 0;
            } else {
                in_string = true;
                string_char = s[i];
            }
            continue;
        }
        if (in_string) continue;
        
        if (s[i] == '{') depth++;
        else if (s[i] == '}') {
            depth--;
            if (depth == 0) return i;
        }
    }
    return -1;
}

static yay_value_t *parse_inline_string(parse_ctx_t *ctx __attribute__((unused)),
                                         const char *s, 
                                         size_t *consumed,
                                         int line_num __attribute__((unused)),
                                         int col __attribute__((unused))) {
    char quote = s[0];
    size_t i = 1;
    size_t cap = 64;
    size_t len = 0;
    char *out = malloc(cap);
    bool escape = false;
    
    while (s[i]) {
        if (escape) {
            if (quote == '"') {
                switch (s[i]) {
                    case '"': out[len++] = '"'; break;
                    case '\\': out[len++] = '\\'; break;
                    case '/': out[len++] = '/'; break;
                    case 'b': out[len++] = '\b'; break;
                    case 'f': out[len++] = '\f'; break;
                    case 'n': out[len++] = '\n'; break;
                    case 'r': out[len++] = '\r'; break;
                    case 't': out[len++] = '\t'; break;
                    case 'u': {
                        if (!s[i+1] || !s[i+2] || !s[i+3] || !s[i+4]) {
                            free(out);
                            return NULL;
                        }
                        char hex[5] = {s[i+1], s[i+2], s[i+3], s[i+4], 0};
                        unsigned int code;
                        sscanf(hex, "%x", &code);
                        if (code < 0x80) {
                            out[len++] = code;
                        } else if (code < 0x800) {
                            out[len++] = 0xC0 | (code >> 6);
                            out[len++] = 0x80 | (code & 0x3F);
                        } else {
                            out[len++] = 0xE0 | (code >> 12);
                            out[len++] = 0x80 | ((code >> 6) & 0x3F);
                            out[len++] = 0x80 | (code & 0x3F);
                        }
                        i += 4;
                        break;
                    }
                    default: out[len++] = s[i]; break;
                }
            } else {
                /* Single quote - only \' and \\ are escapes */
                if (s[i] == '\'' || s[i] == '\\') {
                    out[len++] = s[i];
                } else {
                    out[len++] = '\\';
                    out[len++] = s[i];
                }
            }
            escape = false;
            i++;
            continue;
        }
        
        if (s[i] == '\\') {
            escape = true;
            i++;
            continue;
        }
        
        if (s[i] == quote) {
            *consumed = i + 1;
            out[len] = '\0';
            yay_value_t *v = yay_string(out);
            free(out);
            return v;
        }
        
        if (len + 4 >= cap) {
            cap *= 2;
            out = realloc(out, cap);
        }
        out[len++] = s[i++];
    }
    
    free(out);
    return NULL;
}

static yay_value_t *parse_inline_number(const char *s, size_t *consumed) {
    size_t i = 0;
    bool has_decimal = false;
    
    if (s[i] == '-') i++;
    
    while (s[i] && (isdigit(s[i]) || s[i] == '.')) {
        if (s[i] == '.') has_decimal = true;
        i++;
    }
    
    if (i == 0 || (i == 1 && s[0] == '-')) {
        return NULL;
    }
    
    char *num_str = str_dup_len(s, i);
    *consumed = i;
    
    if (has_decimal) {
        double val = atof(num_str);
        free(num_str);
        return yay_float(val);
    } else {
        bool negative = num_str[0] == '-';
        char *digits = negative ? num_str + 1 : num_str;
        yay_value_t *v = yay_int_from_str(digits, negative);
        free(num_str);
        return v;
    }
}

static yay_value_t *parse_inline_bytes(parse_ctx_t *ctx, const char *s, 
                                        size_t *consumed, int line_num, int col) {
    /* Find closing > */
    const char *end = strchr(s + 1, '>');
    if (!end) return NULL;
    
    /* Validate whitespace in byte literal */
    if (s[1] == ' ') {
        ctx->error = make_error(ctx, line_num, col + 1, 
                               "Unexpected space after \"<\"");
        return NULL;
    }
    if (end > s + 1 && *(end - 1) == ' ') {
        ctx->error = make_error(ctx, line_num, col + (int)(end - s - 1), 
                               "Unexpected space before \">\"");
        return NULL;
    }
    
    size_t hex_len = end - s - 1;
    char *hex = malloc(hex_len + 1);
    size_t hex_out = 0;
    
    for (size_t i = 1; i < (size_t)(end - s); i++) {
        if (s[i] != ' ' && s[i] != '\t') {
            hex[hex_out++] = tolower(s[i]);
        }
    }
    hex[hex_out] = '\0';
    
    if (hex_out % 2 != 0) {
        ctx->error = make_error(ctx, line_num, col, 
                               "Odd number of hex digits in byte literal");
        free(hex);
        return NULL;
    }
    
    yay_value_t *v = yay_bytes_from_hex(hex);
    free(hex);
    *consumed = end - s + 1;
    return v;
}

static yay_value_t *parse_inline_value_impl(parse_ctx_t *ctx, const char *s, 
                                             size_t *consumed, int line_num, int col);

static yay_value_t *parse_inline_value(parse_ctx_t *ctx, const char *s, 
                                        size_t *consumed, int line_num, int col) {
    /* Skip whitespace */
    while (*s == ' ') { s++; col++; }
    
    /* Validate syntax before parsing (only for top-level arrays/objects) */
    if (s[0] == '[' || s[0] == '{') {
        if (!validate_inline_syntax(ctx, s, line_num, col)) {
            return NULL;
        }
    }
    
    return parse_inline_value_impl(ctx, s, consumed, line_num, col);
}

static yay_value_t *parse_inline_value_impl(parse_ctx_t *ctx, const char *s, 
                                             size_t *consumed, int line_num, int col) {
    /* Skip whitespace */
    while (*s == ' ') { s++; col++; }
    
    if (s[0] == '[') {
        int end = find_matching_bracket(s);
        if (end < 0) return NULL;
        
        yay_value_t *arr = yay_array();
        const char *inner = s + 1;
        int inner_col = col + 1;
        
        while (*inner && *inner != ']') {
            while (*inner == ' ' || *inner == ',') { inner++; inner_col++; }
            if (*inner == ']') break;
            
            size_t item_consumed;
            yay_value_t *item = parse_inline_value_impl(ctx, inner, &item_consumed, 
                                                         line_num, inner_col);
            if (!item) {
                yay_free(arr);
                return NULL;
            }
            yay_array_push(arr, item);
            inner += item_consumed;
            inner_col += item_consumed;
        }
        
        *consumed = end + 1;
        return arr;
    }
    
    if (s[0] == '{') {
        int end = find_matching_brace(s);
        if (end < 0) return NULL;
        
        yay_value_t *obj = yay_object();
        const char *inner = s + 1;
        int inner_col = col + 1;
        
        while (*inner && *inner != '}') {
            while (*inner == ' ' || *inner == ',') { inner++; inner_col++; }
            if (*inner == '}') break;
            
            /* Parse key */
            char *key = NULL;
            if (*inner == '"' || *inner == '\'') {
                size_t key_consumed;
                yay_value_t *key_val = parse_inline_string(ctx, inner, &key_consumed,
                                                           line_num, inner_col);
                if (!key_val) {
                    yay_free(obj);
                    return NULL;
                }
                key = str_dup(key_val->data.string);
                yay_free(key_val);
                inner += key_consumed;
                inner_col += key_consumed;
            } else {
                /* Unquoted key - must start with alphanumeric or underscore */
                const char *key_start = inner;
                if (!isalnum(*inner) && *inner != '_') {
                    /* Invalid key character at start */
                    ctx->error = make_error(ctx, line_num, col, "Invalid key");
                    yay_free(obj);
                    return NULL;
                }
                while (*inner && (isalnum(*inner) || *inner == '_' || *inner == '-')) { inner++; inner_col++; }
                key = str_dup_len(key_start, inner - key_start);
            }
            
            /* Skip colon */
            while (*inner == ' ') { inner++; inner_col++; }
            if (*inner != ':') {
                ctx->error = make_error(ctx, line_num, col, "Expected colon after key");
                free(key);
                yay_free(obj);
                return NULL;
            }
            inner++; inner_col++;
            while (*inner == ' ') { inner++; inner_col++; }
            
            /* Parse value */
            size_t val_consumed;
            yay_value_t *val = parse_inline_value(ctx, inner, &val_consumed,
                                                   line_num, inner_col);
            if (!val) {
                free(key);
                yay_free(obj);
                return NULL;
            }
            
            yay_object_set(obj, key, val);
            free(key);
            inner += val_consumed;
            inner_col += val_consumed;
        }
        
        *consumed = end + 1;
        return obj;
    }
    
    if (s[0] == '<') {
        return parse_inline_bytes(ctx, s, consumed, line_num, col);
    }
    
    if (s[0] == '"' || s[0] == '\'') {
        return parse_inline_string(ctx, s, consumed, line_num, col);
    }
    
    if (strncmp(s, "true", 4) == 0 && !isalnum(s[4])) {
        *consumed = 4;
        return yay_bool(true);
    }
    
    if (strncmp(s, "false", 5) == 0 && !isalnum(s[5])) {
        *consumed = 5;
        return yay_bool(false);
    }
    
    if (strncmp(s, "null", 4) == 0 && !isalnum(s[4])) {
        *consumed = 4;
        return yay_null();
    }
    
    if (strncmp(s, "nan", 3) == 0 && !isalnum(s[3])) {
        *consumed = 3;
        return yay_float(NAN);
    }
    
    if (strncmp(s, "infinity", 8) == 0 && !isalnum(s[8])) {
        *consumed = 8;
        return yay_float(INFINITY);
    }
    
    if (strncmp(s, "-infinity", 9) == 0 && !isalnum(s[9])) {
        *consumed = 9;
        return yay_float(-INFINITY);
    }
    
    /* Try number */
    return parse_inline_number(s, consumed);
}

static yay_value_t *parse_inline_array(parse_ctx_t *ctx, const char *s,
                                        int line_num, int col) {
    size_t consumed;
    return parse_inline_value(ctx, s, &consumed, line_num, col);
}

/* ============================================================================
 * Byte Array Parsing
 * ============================================================================ */

static yay_value_t *parse_angle_bytes(parse_ctx_t *ctx, const char *s,
                                       int line_num, int col) {
    if (strcmp(s, "<>") == 0) {
        return yay_bytes(NULL, 0);
    }
    
    size_t len = strlen(s);
    if (s[0] != '<') {
        return NULL;
    }
    
    /* Check for unclosed angle bracket */
    if (len < 2 || s[len-1] != '>') {
        ctx->error = make_error(ctx, line_num, col,
                               "Unmatched angle bracket");
        return NULL;
    }
    
    /* Validate whitespace */
    if (s[1] == ' ') {
        ctx->error = make_error(ctx, line_num, col + 1,
                               "Unexpected space after \"<\"");
        return NULL;
    }
    if (len > 2 && s[len-2] == ' ') {
        ctx->error = make_error(ctx, line_num, col + (int)len - 2,
                               "Unexpected space before \">\"");
        return NULL;
    }
    
    /* Extract hex, removing spaces, and validate hex digits */
    char *hex = malloc(len);
    size_t hex_len = 0;
    
    for (size_t i = 1; i < len - 1; i++) {
        if (s[i] != ' ') {
            char c = s[i];
            /* Reject uppercase hex digits */
            if (c >= 'A' && c <= 'F') {
                ctx->error = make_error(ctx, line_num, col + (int)i,
                                       "Uppercase hex digit (use lowercase)");
                free(hex);
                return NULL;
            }
            /* Validate hex digit */
            if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))) {
                ctx->error = make_error(ctx, line_num, col, "Invalid hex digit");
                free(hex);
                return NULL;
            }
            hex[hex_len++] = c;
        }
    }
    hex[hex_len] = '\0';
    
    if (hex_len % 2 != 0) {
        ctx->error = make_error(ctx, line_num, col,
                               "Odd number of hex digits in byte literal");
        free(hex);
        return NULL;
    }
    
    yay_value_t *v = yay_bytes_from_hex(hex);
    free(hex);
    return v;
}

/* ============================================================================
 * Comment Handling
 * ============================================================================ */

/* Strip inline comment from a string (modifies in place, returns pointer to start) */
static char *strip_inline_comment(char *s) {
    int in_double = 0;
    int in_single = 0;
    int escape = 0;
    
    for (char *p = s; *p; p++) {
        if (escape) {
            escape = 0;
            continue;
        }
        if (*p == '\\') {
            escape = 1;
            continue;
        }
        if (*p == '"' && !in_single) {
            in_double = !in_double;
        } else if (*p == '\'' && !in_double) {
            in_single = !in_single;
        } else if (*p == '#' && !in_double && !in_single) {
            /* Trim trailing whitespace before comment */
            char *end = p;
            while (end > s && *(end-1) == ' ') end--;
            *end = '\0';
            return s;
        }
    }
    return s;
}

/* ============================================================================
 * Scalar Parsing
 * ============================================================================ */

static yay_value_t *parse_scalar_impl(parse_ctx_t *ctx, const char *s,
                                       int line_num, int col);

static yay_value_t *parse_scalar(parse_ctx_t *ctx, const char *s,
                                  int line_num, int col) {
    /* Strip inline comments first */
    char *s_copy = str_dup(s);
    strip_inline_comment(s_copy);
    yay_value_t *result = parse_scalar_impl(ctx, s_copy, line_num, col);
    free(s_copy);
    return result;
}

static yay_value_t *parse_scalar_impl(parse_ctx_t *ctx, const char *s,
                                       int line_num, int col) {
    /* Keywords */
    if (strcmp(s, "null") == 0) return yay_null();
    if (strcmp(s, "true") == 0) return yay_bool(true);
    if (strcmp(s, "false") == 0) return yay_bool(false);
    if (strcmp(s, "nan") == 0) return yay_float(NAN);
    if (strcmp(s, "infinity") == 0) return yay_float(INFINITY);
    if (strcmp(s, "-infinity") == 0) return yay_float(-INFINITY);
    
    /* Numbers */
    if (is_float_str(s)) {
        return parse_number_with_validation(ctx, s, line_num, col);
    }
    if (is_integer_str(s)) {
        return parse_number_with_validation(ctx, s, line_num, col);
    }
    
    /* Double-quoted string */
    if (s[0] == '"') {
        size_t slen = strlen(s);
        if (slen < 2 || s[slen-1] != '"') {
            /* Unterminated string - report at end of string */
            ctx->error = make_error(ctx, line_num, col + (int)(slen > 0 ? slen - 1 : 0),
                                   "Unterminated string");
            return NULL;
        }
        return parse_double_quoted_string(ctx, s, line_num, col);
    }
    
    /* Single-quoted string */
    if (s[0] == '\'') {
        size_t slen = strlen(s);
        if (slen < 2 || s[slen-1] != '\'') {
            /* Unterminated string - report at end of string */
            ctx->error = make_error(ctx, line_num, col + (int)(slen > 0 ? slen - 1 : 0),
                                   "Unterminated string");
            return NULL;
        }
        return parse_single_quoted_string(s);
    }
    
    /* Inline array - must close on same line */
    if (s[0] == '[') {
        if (!strchr(s, ']')) {
            ctx->error = make_error(ctx, line_num, col,
                                   "Unexpected newline in inline array");
            return NULL;
        }
        return parse_inline_array(ctx, s, line_num, col);
    }
    
    /* Inline object - must close on same line */
    if (s[0] == '{') {
        if (!strchr(s, '}')) {
            ctx->error = make_error(ctx, line_num, col,
                                   "Unexpected newline in inline object");
            return NULL;
        }
        /* Validate inline syntax first */
        if (!validate_inline_syntax(ctx, s, line_num, col)) {
            return NULL;
        }
        size_t consumed;
        return parse_inline_value_impl(ctx, s, &consumed, line_num, col);
    }
    
    /* Inline bytes */
    if (s[0] == '<') {
        yay_value_t *bytes = parse_angle_bytes(ctx, s, line_num, col);
        if (ctx->error) {
            return NULL;
        }
        return bytes;
    }
    
    /* Bare words are not valid - strings must be quoted */
    char first_char = s[0] ? s[0] : '?';
    char msg[64];
    snprintf(msg, sizeof(msg), "Unexpected character \"%c\"", first_char);
    ctx->error = make_error(ctx, line_num, col, msg);
    return NULL;
}

/* ============================================================================
 * Object Parsing
 * ============================================================================ */

static yay_value_t *parse_nested_object(parse_ctx_t *ctx, size_t *idx, 
                                         int base_indent);

static yay_value_t *parse_object_property_value(parse_ctx_t *ctx, size_t *idx,
                                                 token_t *t, const char *v_part,
                                                 int v_col) {
    /* Empty object */
    if (strcmp(v_part, "{}") == 0) {
        (*idx)++;
        return yay_object();
    }
    
    /* Block string starting on same line: key: ` content */
    if (v_part[0] == '`') {
        /* In property context, block leader must be alone or followed by newline */
        if (strlen(v_part) > 1) {
            ctx->error = calloc(1, sizeof(yay_error_t));
            ctx->error->message = str_dup("Expected newline after block leader in property");
            return NULL;
        }
        /* Block leader alone - content starts on next line */
        return parse_property_block_string_indent(ctx, idx, "", t->indent);
    }
    
    /* Block bytes starting on same line: key: > hex */
    if (v_part[0] == '>' && !strchr(v_part, '<')) {
        /* In property context, block leader can be followed by comment but not hex */
        if (strlen(v_part) > 1) {
            /* Check if it's just whitespace and/or comment */
            const char *after_leader = v_part + 1;
            while (*after_leader == ' ') after_leader++;
            if (*after_leader != '\0' && *after_leader != '#') {
                /* Has non-comment content on same line */
                ctx->error = calloc(1, sizeof(yay_error_t));
                ctx->error->message = str_dup("Expected newline after block leader in property");
                return NULL;
            }
        }
        return parse_property_block_bytes(ctx, idx, v_part);
    }
    
    /* Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line */
    
    /* Inline value */
    if (strlen(v_part) > 0) {
        (*idx)++;
        yay_value_t *value = parse_scalar(ctx, v_part, t->line_num, v_col);
        if (ctx->error) {
            return NULL;
        }
        return value;
    }
    
    /* Nested content */
    int colon_idx = find_colon_outside_quotes(t->text);
    (*idx)++;
    size_t j = skip_breaks_and_stops(ctx, *idx);
    
    if (j >= ctx->token_count) {
        /* Empty property with no nested content is invalid */
        ctx->error = make_error(ctx, t->line_num, t->col + colon_idx + 1,
                               "Expected value after property");
        return NULL;
    }
    
    token_t *next_t = &ctx->tokens[j];
    
    /* Named array - pass next_t->indent so array stops at items below this level */
    if (next_t->type == TOKEN_START && strcmp(next_t->text, "- ") == 0) {
        *idx = j;
        return parse_multiline_array_impl(ctx, idx, next_t->indent);
    }
    
    /* Note: Block string/bytes leaders must be on the same line as the property key */
    /* Block string leader on next line is invalid */
    if (next_t->type == TOKEN_TEXT && strcmp(next_t->text, "`") == 0) {
        ctx->error = make_error(ctx, next_t->line_num, 0, "Unexpected indent");
        return NULL;
    }
    
    /* Block bytes leader on next line is invalid */
    if (next_t->type == TOKEN_TEXT && next_t->text[0] == '>' && 
        !strchr(next_t->text, '<')) {
        ctx->error = make_error(ctx, next_t->line_num, 0, "Unexpected indent");
        return NULL;
    }
    
    /* Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line */
    
    /* Reject inline values on separate line (they look like keys starting with special chars) */
    if (next_t->type == TOKEN_TEXT && next_t->indent > t->indent) {
        const char *trimmed = next_t->text;
        while (*trimmed == ' ') trimmed++;
        if (trimmed[0] == '[' || trimmed[0] == '{' || trimmed[0] == '<') {
            ctx->error = make_error(ctx, next_t->line_num, 0, "Unexpected indent");
            return NULL;
        }
        /* Reject numbers on separate line */
        if ((trimmed[0] >= '0' && trimmed[0] <= '9') || 
            (trimmed[0] == '-' && trimmed[1] >= '0' && trimmed[1] <= '9') ||
            (trimmed[0] == '.' && trimmed[1] >= '0' && trimmed[1] <= '9')) {
            ctx->error = make_error(ctx, next_t->line_num, 0, "Unexpected indent");
            return NULL;
        }
    }
    
    /* Concatenated quoted strings (multiple quoted strings on consecutive lines) */
    if (next_t->type == TOKEN_TEXT && next_t->indent > t->indent) {
        const char *trimmed = next_t->text;
        while (*trimmed == ' ') trimmed++;
        size_t trimmed_len = strlen(trimmed);
        bool is_double_quoted = trimmed_len >= 2 && trimmed[0] == '"' && trimmed[trimmed_len-1] == '"';
        bool is_single_quoted = trimmed_len >= 2 && trimmed[0] == '\'' && trimmed[trimmed_len-1] == '\'';
        
        if (is_double_quoted || is_single_quoted) {
            *idx = j;
            yay_value_t *result = parse_concatenated_strings(ctx, idx, next_t->indent);
            if (result != NULL) {
                return result;
            }
            /* Single string on new line is invalid - fall through to error */
            ctx->error = make_error(ctx, next_t->line_num, 0, "Unexpected indent");
            return NULL;
        }
    }
    
    /* Nested object */
    if (next_t->type == TOKEN_TEXT && next_t->indent > t->indent) {
        *idx = j;
        return parse_nested_object(ctx, idx, next_t->indent);
    }
    
    /* Empty property with no nested content is invalid */
    ctx->error = make_error(ctx, t->line_num, t->col + colon_idx + 1,
                           "Expected value after property");
    return NULL;
}

/* Validate object property syntax (whitespace around colon, key characters) */
static bool validate_object_property(parse_ctx_t *ctx, const char *text, 
                                      int colon_idx, int line_num, int col) {
    /* Check for space before colon */
    if (colon_idx > 0 && text[colon_idx - 1] == ' ') {
        ctx->error = make_error(ctx, line_num, col + colon_idx - 1,
                               "Unexpected space before \":\"");
        return false;
    }
    
    /* Check for space after colon */
    const char *after_colon = text + colon_idx + 1;
    if (*after_colon == '\0') {
        /* Colon at end of line - that's ok (block value follows) */
    } else if (*after_colon != ' ') {
        ctx->error = make_error(ctx, line_num, col + colon_idx,
                               "Expected space after \":\"");
        return false;
    } else if (after_colon[0] == ' ' && after_colon[1] == ' ') {
        ctx->error = make_error(ctx, line_num, col + colon_idx + 2,
                               "Unexpected space after \":\"");
        return false;
    }
    
    /* Validate key characters for unquoted keys */
    if (text[0] != '"' && text[0] != '\'') {
        for (int ki = 0; ki < colon_idx; ki++) {
            char kc = text[ki];
            if (!isalnum(kc) && kc != '_' && kc != '-') {
                ctx->error = make_error(ctx, line_num, col + ki,
                                       "Invalid key character");
                return false;
            }
        }
    }
    
    return true;
}

static yay_value_t *parse_nested_object(parse_ctx_t *ctx, size_t *idx, 
                                         int base_indent) {
    yay_value_t *obj = yay_object();
    
    while (*idx < ctx->token_count) {
        token_t *t = &ctx->tokens[*idx];
        
        if (t->type == TOKEN_STOP || t->type == TOKEN_BREAK) {
            (*idx)++;
            continue;
        }
        
        if (t->type != TOKEN_TEXT) {
            /* START tokens indicate a new list item - break out */
            break;
        }
        
        int colon_idx = find_colon_outside_quotes(t->text);
        if (colon_idx < 0 || t->indent < base_indent) {
            break;
        }
        
        /* Validate object property syntax */
        if (!validate_object_property(ctx, t->text, colon_idx, t->line_num, t->col)) {
            yay_free(obj);
            return NULL;
        }
        
        char *k_raw = str_dup_len(t->text, colon_idx);
        char *key = parse_key_name(k_raw);
        free(k_raw);
        
        const char *v_part = t->text + colon_idx + 1;
        int v_col = t->col + colon_idx + 1;
        while (*v_part == ' ') { v_part++; v_col++; }
        
        if (strlen(key) == 0) {
            free(key);
            (*idx)++;
            continue;
        }
        
        yay_value_t *value = parse_object_property_value(ctx, idx, t, v_part, v_col);
        if (ctx->error) {
            free(key);
            yay_free(obj);
            return NULL;
        }
        
        yay_object_set(obj, key, value);
        free(key);
    }
    
    return obj;
}

/* ============================================================================
 * Multiline Array Parsing
 * ============================================================================ */

/* Check if text starts with "- " (inline bullet) */
static bool is_inline_bullet(const char *text) {
    int i = 0;
    while (text[i] == ' ') i++;
    return text[i] == '-' && text[i + 1] == ' ';
}

/* Validate inline bullet has exactly one space after "-" */
static bool validate_inline_bullet(parse_ctx_t *ctx, const char *text, int line_num, int col) {
    int i = 0;
    while (text[i] == ' ') i++;
    if (text[i] == '-' && text[i + 1] == ' ' && text[i + 2] == ' ') {
        ctx->error = make_error(ctx, line_num, col + i + 2,
                               "Unexpected space after \"-\"");
        return false;
    }
    return true;
}

/* Parse inline bullet value (extract content after "- ") */
static const char *get_inline_bullet_value(const char *text) {
    int i = 0;
    while (text[i] == ' ') i++;
    if (text[i] == '-' && text[i + 1] == ' ') {
        return text + i + 2;
    }
    return text;
}

/* Recursively parse nested inline bullets like "- - - value" */
static yay_value_t *parse_nested_inline_bullet(parse_ctx_t *ctx, const char *text,
                                                int line_num, int col) {
    if (is_inline_bullet(text)) {
        const char *inner_text = get_inline_bullet_value(text);
        yay_value_t *inner_val = parse_nested_inline_bullet(ctx, inner_text, line_num, col + 2);
        if (ctx->error) {
            return NULL;
        }
        yay_value_t *arr = yay_array();
        yay_array_push(arr, inner_val);
        return arr;
    }
    return parse_scalar(ctx, text, line_num, col);
}

/* Forward declaration with min_indent parameter */
static yay_value_t *parse_multiline_array_impl(parse_ctx_t *ctx, size_t *idx, int min_indent);

static yay_value_t *parse_multiline_array(parse_ctx_t *ctx, size_t *idx) {
    return parse_multiline_array_impl(ctx, idx, -1);
}

static yay_value_t *parse_multiline_array_impl(parse_ctx_t *ctx, size_t *idx, int min_indent) {
    yay_value_t *arr = yay_array();
    
    while (*idx < ctx->token_count && 
           ctx->tokens[*idx].type == TOKEN_START &&
           strcmp(ctx->tokens[*idx].text, "- ") == 0) {
        int list_indent = ctx->tokens[*idx].indent;
        /* Stop if we encounter a list item at a lower indent than expected */
        if (min_indent >= 0 && list_indent < min_indent) {
            break;
        }
        (*idx)++;
        
        *idx = skip_breaks(ctx, *idx);
        if (*idx >= ctx->token_count) break;
        
        token_t *next = &ctx->tokens[*idx];
        
        /* Nested array - START token */
        if (next->type == TOKEN_START && strcmp(next->text, "- ") == 0) {
            yay_value_t *nested = parse_multiline_array_impl(ctx, idx, -1);
            if (ctx->error) {
                yay_free(arr);
                return NULL;
            }
            yay_array_push(arr, nested);
        }
        /* Inline bullet (text starts with "- ") - creates nested array */
        else if (next->type == TOKEN_TEXT && is_inline_bullet(next->text)) {
            /* Validate the first inline bullet */
            if (!validate_inline_bullet(ctx, next->text, next->line_num, next->col)) {
                yay_free(arr);
                return NULL;
            }
            
            yay_value_t *nested = yay_array();
            
            /* Collect all inline bullets at this level */
            while (*idx < ctx->token_count && 
                   ctx->tokens[*idx].type == TOKEN_TEXT &&
                   is_inline_bullet(ctx->tokens[*idx].text)) {
                /* Validate each inline bullet */
                if (!validate_inline_bullet(ctx, ctx->tokens[*idx].text,
                                            ctx->tokens[*idx].line_num,
                                            ctx->tokens[*idx].col)) {
                    yay_free(nested);
                    yay_free(arr);
                    return NULL;
                }
                
                const char *val_str = get_inline_bullet_value(ctx->tokens[*idx].text);
                yay_value_t *item = parse_nested_inline_bullet(ctx, val_str, 
                                                  ctx->tokens[*idx].line_num,
                                                  ctx->tokens[*idx].col + 2);
                if (ctx->error) {
                    yay_free(nested);
                    yay_free(arr);
                    return NULL;
                }
                yay_array_push(nested, item);
                (*idx)++;
            }
            
            /* Continue with nested START tokens at deeper indent */
            while (*idx < ctx->token_count &&
                   ctx->tokens[*idx].type == TOKEN_START &&
                   strcmp(ctx->tokens[*idx].text, "- ") == 0 &&
                   ctx->tokens[*idx].indent > list_indent) {
                (*idx)++;
                *idx = skip_breaks(ctx, *idx);
                if (*idx >= ctx->token_count) break;
                
                yay_value_t *sub_val = parse_value(ctx, idx);
                if (ctx->error) {
                    yay_free(nested);
                    yay_free(arr);
                    return NULL;
                }
                yay_array_push(nested, sub_val);
                *idx = skip_stops(ctx, *idx);
            }
            
            yay_array_push(arr, nested);
        }
        /* Object in array */
        else if (next->type == TOKEN_TEXT && find_colon_outside_quotes(next->text) >= 0) {
            /* Use list_indent as base so sibling properties at higher indent are included */
            yay_value_t *obj = parse_nested_object(ctx, idx, list_indent);
            if (ctx->error) {
                yay_free(arr);
                return NULL;
            }
            yay_array_push(arr, obj);
        }
        /* Regular value */
        else if (next->type == TOKEN_TEXT) {
            const char *s = next->text;
            /* Block string in array context - use list_indent as base */
            if (strcmp(s, "`") == 0 || (s[0] == '`' && strlen(s) >= 2 && s[1] == ' ')) {
                const char *first_line = strlen(s) > 2 ? s + 2 : "";
                yay_value_t *value = parse_block_string_impl(ctx, idx, first_line, list_indent);
                if (ctx->error) {
                    yay_free(arr);
                    return NULL;
                }
                yay_array_push(arr, value);
            } else {
                yay_value_t *value = parse_value(ctx, idx);
                if (ctx->error) {
                    yay_free(arr);
                    return NULL;
                }
                yay_array_push(arr, value);
            }
        }
        else {
            (*idx)++;
        }
        
        /* Skip stops and breaks between items */
        *idx = skip_stops(ctx, *idx);
        *idx = skip_breaks(ctx, *idx);
    }
    
    return arr;
}

/* ============================================================================
 * Value Parsing
 * ============================================================================ */

static yay_value_t *parse_value(parse_ctx_t *ctx, size_t *idx) {
    if (*idx >= ctx->token_count) {
        return yay_null();
    }
    
    token_t *t = &ctx->tokens[*idx];
    
    /* Validate text tokens */
    if (t->type == TOKEN_TEXT) {
        if (t->text[0] == ' ') {
            ctx->error = make_error(ctx, t->line_num, t->col,
                                   "Unexpected leading space");
            return NULL;
        }
        if (strcmp(t->text, "$") == 0) {
            ctx->error = make_error(ctx, t->line_num, t->col,
                                   "Unexpected character \"$\"");
            return NULL;
        }
    }
    
    /* Block starts (list items) */
    if (t->type == TOKEN_START && strcmp(t->text, "- ") == 0) {
        return parse_multiline_array(ctx, idx);
    }
    
    /* Text content */
    if (t->type == TOKEN_TEXT) {
        const char *s = t->text;
        
        /* Keywords */
        if (strcmp(s, "null") == 0) { (*idx)++; return yay_null(); }
        if (strcmp(s, "true") == 0) { (*idx)++; return yay_bool(true); }
        if (strcmp(s, "false") == 0) { (*idx)++; return yay_bool(false); }
        if (strcmp(s, "nan") == 0) { (*idx)++; return yay_float(NAN); }
        if (strcmp(s, "infinity") == 0) { (*idx)++; return yay_float(INFINITY); }
        if (strcmp(s, "-infinity") == 0) { (*idx)++; return yay_float(-INFINITY); }
        
        /* Numbers */
        if (is_float_str(s)) {
            yay_value_t *v = parse_number_with_validation(ctx, s, t->line_num, t->col);
            if (ctx->error) return NULL;
            (*idx)++;
            return v;
        }
        if (is_integer_str(s)) {
            yay_value_t *v = parse_number_with_validation(ctx, s, t->line_num, t->col);
            if (ctx->error) return NULL;
            (*idx)++;
            return v;
        }
        
        /* Block string (backtick) */
        if (strcmp(s, "`") == 0 || (s[0] == '`' && strlen(s) >= 2 && s[1] == ' ')) {
            const char *first_line = strlen(s) > 2 ? s + 2 : "";
            return parse_block_string(ctx, idx, first_line);
        }
        
        /* Block bytes (>) */
        if (s[0] == '>' && !strchr(s, '<')) {
            return parse_block_bytes(ctx, idx);
        }
        
        /* Quoted string */
        if (s[0] == '"' && strlen(s) > 1) {
            size_t slen = strlen(s);
            if (s[slen-1] != '"') {
                /* Unterminated string - report at end of string */
                ctx->error = make_error(ctx, t->line_num, t->col + (int)(slen > 0 ? slen - 1 : 0),
                                       "Unterminated string");
                return NULL;
            }
            (*idx)++;
            return parse_double_quoted_string(ctx, s, t->line_num, t->col);
        }
        if (s[0] == '\'' && strlen(s) > 1) {
            size_t slen = strlen(s);
            if (s[slen-1] != '\'') {
                /* Unterminated string - report at end of string */
                ctx->error = make_error(ctx, t->line_num, t->col + (int)(slen > 0 ? slen - 1 : 0),
                                       "Unterminated string");
                return NULL;
            }
            (*idx)++;
            return parse_single_quoted_string(s);
        }
        
        /* Inline array - must close on same line */
        if (s[0] == '[') {
            if (!strchr(s, ']')) {
                ctx->error = make_error(ctx, t->line_num, t->col,
                                       "Unexpected newline in inline array");
                return NULL;
            }
            (*idx)++;
            return parse_inline_array(ctx, s, t->line_num, t->col);
        }
        
        /* Inline object - must close on same line */
        if (s[0] == '{') {
            if (!strchr(s, '}')) {
                ctx->error = make_error(ctx, t->line_num, t->col,
                                       "Unexpected newline in inline object");
                return NULL;
            }
            size_t consumed;
            (*idx)++;
            return parse_inline_value(ctx, s, &consumed, t->line_num, t->col);
        }
        
        /* Inline bytes */
        if (s[0] == '<') {
            (*idx)++;
            yay_value_t *bytes = parse_angle_bytes(ctx, s, t->line_num, t->col);
            if (ctx->error) {
                return NULL;
            }
            return bytes;
        }
        
        /* Key:value pair */
        int colon_idx = find_colon_outside_quotes(s);
        if (colon_idx >= 0) {
            char *k_raw = str_dup_len(s, colon_idx);
            char *key = parse_key_name(k_raw);
            free(k_raw);
            
            const char *v_part = s + colon_idx + 1;
            int v_col = t->col + colon_idx + 1;
            while (*v_part == ' ') { v_part++; v_col++; }
            
            yay_value_t *obj = yay_object();
            yay_value_t *value = parse_object_property_value(ctx, idx, t, v_part, v_col);
            if (ctx->error) {
                free(key);
                yay_free(obj);
                return NULL;
            }
            
            yay_object_set(obj, key, value);
            free(key);
            return obj;
        }
        
        /* Fall back to scalar */
        (*idx)++;
        return parse_scalar(ctx, s, t->line_num, t->col);
    }
    
    (*idx)++;
    return yay_null();
}

/* ============================================================================
 * Root Parsing
 * ============================================================================ */

static yay_value_t *parse_root_object(parse_ctx_t *ctx, size_t *idx) {
    yay_value_t *obj = yay_object();
    
    while (*idx < ctx->token_count) {
        token_t *t = &ctx->tokens[*idx];
        
        if (t->type == TOKEN_STOP || t->type == TOKEN_BREAK) {
            (*idx)++;
            continue;
        }
        
        if (t->type != TOKEN_TEXT || t->indent != 0) {
            (*idx)++;
            continue;
        }
        
        int colon_idx = find_colon_outside_quotes(t->text);
        if (colon_idx < 0) {
            (*idx)++;
            continue;
        }
        
        /* Validate object property syntax */
        if (!validate_object_property(ctx, t->text, colon_idx, t->line_num, t->col)) {
            yay_free(obj);
            return NULL;
        }
        
        char *k_raw = str_dup_len(t->text, colon_idx);
        char *key = parse_key_name(k_raw);
        free(k_raw);
        
        const char *v_part = t->text + colon_idx + 1;
        int v_col = t->col + colon_idx + 1;
        while (*v_part == ' ') { v_part++; v_col++; }
        
        yay_value_t *value = parse_object_property_value(ctx, idx, t, v_part, v_col);
        if (ctx->error) {
            free(key);
            yay_free(obj);
            return NULL;
        }
        
        yay_object_set(obj, key, value);
        free(key);
    }
    
    return obj;
}

static yay_value_t *parse_root(parse_ctx_t *ctx) {
    size_t i = skip_breaks_and_stops(ctx, 0);
    
    if (i >= ctx->token_count) {
        /* No value found - this is an error */
        ctx->error = calloc(1, sizeof(yay_error_t));
        if (ctx->filename) {
            char buf[512];
            snprintf(buf, sizeof(buf), "No value found in document <%s>", ctx->filename);
            ctx->error->message = str_dup(buf);
        } else {
            ctx->error->message = str_dup("No value found in document");
        }
        return NULL;
    }
    
    token_t *t = &ctx->tokens[i];
    
    /* Check for unexpected indent at root */
    if (t->type == TOKEN_TEXT && t->indent > 0) {
        ctx->error = make_error(ctx, t->line_num, 0, "Unexpected indent");
        return NULL;
    }
    
    /* Detect root object (key: value at indent 0, not inline object) */
    if (t->type == TOKEN_TEXT && t->text[0] != '{' &&
        find_colon_outside_quotes(t->text) >= 0 && t->indent == 0) {
        yay_value_t *value = parse_root_object(ctx, &i);
        if (ctx->error) {
            return NULL;
        }
        
        /* Check for extra content */
        size_t j = skip_breaks_and_stops(ctx, i);
        if (j < ctx->token_count) {
            token_t *extra = &ctx->tokens[j];
            ctx->error = make_error(ctx, extra->line_num, extra->col,
                                   "Unexpected extra content");
            yay_free(value);
            return NULL;
        }
        
        return value;
    }
    
    /* Parse as single value */
    yay_value_t *value = parse_value(ctx, &i);
    if (ctx->error) {
        return NULL;
    }
    
    /* Check for extra content */
    size_t j = skip_breaks_and_stops(ctx, i);
    if (j < ctx->token_count) {
        token_t *extra = &ctx->tokens[j];
        ctx->error = make_error(ctx, extra->line_num, extra->col,
                               "Unexpected extra content");
        yay_free(value);
        return NULL;
    }
    
    return value;
}

/* ============================================================================
 * Public API
 * ============================================================================ */

yay_result_t yay_parse(const char *source, size_t length, const char *filename) {
    yay_result_t result = {NULL, NULL};
    
    if (!source) {
        result.error = calloc(1, sizeof(yay_error_t));
        result.error->message = str_dup("NULL source");
        return result;
    }
    
    if (length == 0) {
        length = strlen(source);
    }
    
    /* Initialize context */
    parse_ctx_t ctx = {0};
    ctx.filename = filename;
    ctx.source = source;
    ctx.source_len = length;
    ctx.line_capacity = 64;
    ctx.lines = calloc(ctx.line_capacity, sizeof(scan_line_t));
    ctx.token_capacity = 64;
    ctx.tokens = calloc(ctx.token_capacity, sizeof(token_t));
    
    /* Phase 1: Scan */
    if (!scan(&ctx)) {
        result.error = ctx.error;
        goto cleanup;
    }
    
    /* Phase 2: Outline lex */
    outline_lex(&ctx);
    
    /* Phase 3: Parse */
    result.value = parse_root(&ctx);
    if (ctx.error) {
        result.error = ctx.error;
        yay_free(result.value);
        result.value = NULL;
    }
    
cleanup:
    /* Free scan lines */
    for (size_t i = 0; i < ctx.line_count; i++) {
        free(ctx.lines[i].line);
        free(ctx.lines[i].leader);
    }
    free(ctx.lines);
    
    /* Free tokens */
    for (size_t i = 0; i < ctx.token_count; i++) {
        free(ctx.tokens[i].text);
    }
    free(ctx.tokens);
    
    return result;
}
