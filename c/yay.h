/*
 * YAY Parser - C Implementation
 * YAY is Yet Another YAML - a data serialization format
 *
 * This header defines the public API for parsing YAY documents.
 */

#ifndef YAY_H
#define YAY_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Value Types
 * ============================================================================ */

typedef enum {
    YAY_NULL,       /* null value */
    YAY_BOOL,       /* boolean: true or false */
    YAY_INT,        /* big integer (arbitrary precision) */
    YAY_FLOAT,      /* 64-bit floating point */
    YAY_STRING,     /* UTF-8 string */
    YAY_BYTES,      /* byte array */
    YAY_ARRAY,      /* array of values */
    YAY_OBJECT      /* key-value object */
} yay_type_t;

/* Forward declaration */
typedef struct yay_value yay_value_t;

/* Big integer representation (simple string-based for arbitrary precision) */
typedef struct {
    char *digits;   /* String of digits (without sign) */
    bool negative;  /* true if negative */
} yay_bigint_t;

/* Object key-value pair */
typedef struct {
    char *key;
    yay_value_t *value;
} yay_pair_t;

/* YAY value structure */
struct yay_value {
    yay_type_t type;
    union {
        bool boolean;
        yay_bigint_t bigint;
        double number;
        char *string;
        struct {
            uint8_t *data;
            size_t length;
        } bytes;
        struct {
            yay_value_t **items;
            size_t length;
            size_t capacity;
        } array;
        struct {
            yay_pair_t *pairs;
            size_t length;
            size_t capacity;
        } object;
    } data;
};

/* ============================================================================
 * Error Handling
 * ============================================================================ */

typedef struct {
    char *message;
    int line;       /* 1-based line number */
    int column;     /* 1-based column number */
} yay_error_t;

/* ============================================================================
 * Parse Result
 * ============================================================================ */

typedef struct {
    yay_value_t *value;
    yay_error_t *error;
} yay_result_t;

/* ============================================================================
 * Public API
 * ============================================================================ */

/**
 * Parse a YAY document from a string.
 *
 * @param source    The YAY source string (UTF-8)
 * @param length    Length of the source string (or 0 for null-terminated)
 * @param filename  Optional filename for error messages (can be NULL)
 * @return          Parse result containing either value or error
 */
yay_result_t yay_parse(const char *source, size_t length, const char *filename);

/**
 * Free a YAY value and all its children.
 *
 * @param value     The value to free (can be NULL)
 */
void yay_free(yay_value_t *value);

/**
 * Free a YAY error.
 *
 * @param error     The error to free (can be NULL)
 */
void yay_error_free(yay_error_t *error);

/**
 * Free a parse result (frees both value and error if present).
 *
 * @param result    The result to free
 */
void yay_result_free(yay_result_t *result);

/* ============================================================================
 * Value Constructors (for testing)
 * ============================================================================ */

yay_value_t *yay_null(void);
yay_value_t *yay_bool(bool value);
yay_value_t *yay_int_from_str(const char *digits, bool negative);
yay_value_t *yay_int(int64_t value);
yay_value_t *yay_float(double value);
yay_value_t *yay_string(const char *str);
yay_value_t *yay_string_len(const char *str, size_t len);
yay_value_t *yay_bytes(const uint8_t *data, size_t length);
yay_value_t *yay_bytes_from_hex(const char *hex);
yay_value_t *yay_array(void);
yay_value_t *yay_object(void);

/* Array operations - returns array for chaining */
yay_value_t *yay_array_push(yay_value_t *array, yay_value_t *item);

/* Object operations - returns object for chaining */
yay_value_t *yay_object_set(yay_value_t *object, const char *key, yay_value_t *value);

/* Batch constructors (backing functions for macros below) */
yay_value_t *yay_array_of(yay_value_t **items, size_t count);
yay_value_t *yay_object_of(void **kvs, size_t count);

/* Convenience macros for building arrays and objects from inline lists.
 *
 *   YAY_ARRAY(yay_int(1), yay_string("two"), yay_float(3.0))
 *   YAY_OBJECT("a", yay_int(1), "b", yay_string("two"))
 */
#define YAY_ARRAY(...) \
    yay_array_of( \
        (yay_value_t *[]){__VA_ARGS__}, \
        sizeof((yay_value_t *[]){__VA_ARGS__}) / sizeof(yay_value_t *))

#define YAY_OBJECT(...) \
    yay_object_of( \
        (void *[]){__VA_ARGS__}, \
        sizeof((void *[]){__VA_ARGS__}) / sizeof(void *))

/* ============================================================================
 * Value Comparison (for testing)
 * ============================================================================ */

/**
 * Compare two YAY values for equality.
 *
 * @param a         First value
 * @param b         Second value
 * @return          true if values are equal, false otherwise
 */
bool yay_equal(const yay_value_t *a, const yay_value_t *b);

/**
 * Convert a YAY value to a string representation (for debugging).
 *
 * @param value     The value to convert
 * @return          Allocated string (caller must free)
 */
char *yay_to_string(const yay_value_t *value);

#ifdef __cplusplus
}
#endif

#endif /* YAY_H */
