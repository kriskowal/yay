;;; YAY Test Runner for Scheme (Guile version)
;;; Usage: guile run-tests.scm

;; Import SRFI-11 for let-values
(use-modules (srfi srfi-11))

;; Load the parser
(load "yay-parser.scm")

;; Check if string starts with UTF-8 BOM bytes
(define (has-utf8-bom? raw)
  (and (>= (string-length raw) 3)
       (char=? (string-ref raw 0) (integer->char #xEF))
       (char=? (string-ref raw 1) (integer->char #xBB))
       (char=? (string-ref raw 2) (integer->char #xBF))))

;; Simple UTF-8 decoder from Latin-1 bytes
(define (utf8-decode s)
  (let loop ((i 0) (acc '()))
    (if (>= i (string-length s))
        (list->string (reverse acc))
        (let ((b (char->integer (string-ref s i))))
          (cond
           ;; ASCII
           ((< b #x80)
            (loop (+ i 1) (cons (string-ref s i) acc)))
           ;; 2-byte sequence
           ((and (>= b #xC0) (< b #xE0) (< (+ i 1) (string-length s)))
            (let ((b2 (char->integer (string-ref s (+ i 1)))))
              (loop (+ i 2) 
                    (cons (integer->char (+ (* (- b #xC0) 64) (- b2 #x80))) acc))))
           ;; 3-byte sequence
           ((and (>= b #xE0) (< b #xF0) (< (+ i 2) (string-length s)))
            (let ((b2 (char->integer (string-ref s (+ i 1))))
                  (b3 (char->integer (string-ref s (+ i 2)))))
              (loop (+ i 3)
                    (cons (integer->char (+ (* (- b #xE0) 4096) 
                                            (* (- b2 #x80) 64) 
                                            (- b3 #x80))) acc))))
           ;; 4-byte sequence
           ((and (>= b #xF0) (< b #xF8) (< (+ i 3) (string-length s)))
            (let ((b2 (char->integer (string-ref s (+ i 1))))
                  (b3 (char->integer (string-ref s (+ i 2))))
                  (b4 (char->integer (string-ref s (+ i 3)))))
              (loop (+ i 4)
                    (cons (integer->char (+ (* (- b #xF0) 262144)
                                            (* (- b2 #x80) 4096)
                                            (* (- b3 #x80) 64)
                                            (- b4 #x80))) acc))))
           ;; Invalid - skip
           (else (loop (+ i 1) acc)))))))

;; Read entire file as string, preserving BOM if present
(define (read-file path)
  (call-with-input-file path
    (lambda (port)
      ;; Read in Latin-1 to preserve raw bytes, then convert
      (set-port-encoding! port "ISO-8859-1")
      (let loop ((chars '()))
        (let ((c (read-char port)))
          (if (eof-object? c)
              (let ((raw (list->string (reverse chars))))
                (if (has-utf8-bom? raw)
                    ;; Has BOM - prepend U+FEFF and decode rest as UTF-8
                    (string-append (string #\xFEFF)
                                   (utf8-decode (substring raw 3 (string-length raw))))
                    ;; No BOM - decode as UTF-8
                    (utf8-decode raw)))
              (loop (cons c chars))))))))

;; Trim trailing whitespace
(define (string-trim-end s)
  (let loop ((i (string-length s)))
    (if (or (<= i 0)
            (let ((c (string-ref s (- i 1))))
              (not (or (char=? c #\newline)
                       (char=? c #\return)
                       (char=? c #\space)
                       (char=? c #\tab)))))
        (substring s 0 i)
        (loop (- i 1)))))

;; Test case structure
(define (make-test name yay-content expected-str)
  (list name yay-content expected-str))

(define (test-name t) (car t))
(define (test-yay t) (cadr t))
(define (test-expected-str t) (caddr t))

;; ============================================================================
;; Fixture parsing using Scheme's read with preprocessing
;; ============================================================================
;; The .scm fixture format has some differences from standard Scheme:
;; - 'null represents the null value (reads as (quote null), converted to 'null)
;; - Strings may contain \/ escape (preprocessed to / before read)
;; - Bytevectors are (bytevector 1 2 3)

;; Preprocess fixture string to handle \/ escape
;; Replace \/ with a placeholder, then restore after read
(define (preprocess-fixture-escapes s)
  ;; Replace \/ with just / since Scheme doesn't need the escape
  ;; We do this carefully to avoid breaking other escapes
  (let ((len (string-length s)))
    (let loop ((i 0) (acc '()))
      (if (>= i len)
          (list->string (reverse acc))
          (let ((ch (string-ref s i)))
            (if (and (char=? ch #\\)
                     (< (+ i 1) len)
                     (char=? (string-ref s (+ i 1)) #\/))
                ;; Skip the backslash, keep the /
                (loop (+ i 2) (cons #\/ acc))
                (loop (+ i 1) (cons ch acc))))))))

;; Convert a Scheme value read from .scm fixture to YAY internal representation
(define (fixture->yay-value v)
  (cond
   ;; (quote null) -> 'null
   ((and (pair? v) (eq? (car v) 'quote) (eq? (cadr v) 'null))
    'null)
   ;; 'null symbol directly
   ((eq? v 'null) 'null)
   ((boolean? v) v)
   ((number? v) v)
   ((string? v) v)
   ;; Bytevector: (bytevector 1 2 3) -> (bytevector . (1 2 3))
   ((and (pair? v) (eq? (car v) 'bytevector))
    (cons 'bytevector (cdr v)))
   ;; Vector (array)
   ((vector? v)
    (list->vector (map fixture->yay-value (vector->list v))))
   ;; Alist (object) - list of pairs
   ((and (list? v) (or (null? v) (pair? (car v))))
    (map (lambda (pair)
           (cons (car pair) (fixture->yay-value (cdr pair))))
         v))
   (else v)))

;; Run a single test
(define (run-single-test test)
  (let ((name (test-name test))
        (yay-content (test-yay test))
        (expected-str (test-expected-str test)))
    (catch #t
      (lambda ()
        (let* ((parsed (parse-yay yay-content))
               ;; Preprocess to handle \/ escape, then use Scheme's read
               (preprocessed (preprocess-fixture-escapes expected-str))
               (expected-raw (call-with-input-string preprocessed read))
               (expected (fixture->yay-value expected-raw)))
          (if (yay-equal? parsed expected)
              (begin
                (display "PASS: ")
                (display name)
                (newline)
                #t)
              (begin
                (display "FAIL: ")
                (display name)
                (newline)
                (display "  Expected: ")
                (display (yay->scheme-string expected))
                (newline)
                (display "  Actual:   ")
                (display (yay->scheme-string parsed))
                (newline)
                #f))))
      (lambda (key . args)
        (display "ERROR: ")
        (display name)
        (display " - ")
        (display key)
        (display ": ")
        (for-each (lambda (a) (display a) (display " ")) args)
        (newline)
        #f))))

;; Define all test cases inline (loaded from files at runtime)
(define test-cases '())
(define error-test-cases '())

;; Add a test case
(define (add-test! name yay expected)
  (set! test-cases (cons (make-test name yay expected) test-cases)))

;; Add an error test case
(define (add-error-test! name nay-content)
  (set! error-test-cases (cons (cons name nay-content) error-test-cases)))

;; Run all tests
(define (run-all-tests)
  (display "Running ")
  (display (length test-cases))
  (display " tests")
  (newline)
  (newline)
  (let loop ((tests (reverse test-cases)) (passed 0) (failed 0))
    (if (null? tests)
        (values passed failed)
        (let ((result (run-single-test (car tests))))
          (loop (cdr tests)
                (if result (+ passed 1) passed)
                (if result failed (+ failed 1)))))))

;; Run a single error test (should fail to parse)
(define (run-single-error-test test)
  (let ((name (car test))
        (nay-content (cdr test)))
    (catch #t
      (lambda ()
        ;; If parse succeeds, the test fails
        (parse-yay nay-content)
        (display "FAIL: ")
        (display name)
        (display " (expected error, but parsed successfully)")
        (newline)
        #f)
      (lambda (key . args)
        ;; Parse failed as expected
        (display "PASS: ")
        (display name)
        (newline)
        #t))))

;; Run all error tests
(define (run-all-error-tests)
  (display "Running ")
  (display (length error-test-cases))
  (display " error tests")
  (newline)
  (newline)
  (let loop ((tests (reverse error-test-cases)) (passed 0) (failed 0))
    (if (null? tests)
        (values passed failed)
        (let ((result (run-single-error-test (car tests))))
          (loop (cdr tests)
                (if result (+ passed 1) passed)
                (if result failed (+ failed 1)))))))

;; Load tests from directory
(define (load-tests-from-dir test-dir)
  ;; List of test base names (without extension)
  ;; These are the tests that have both .yay and .scm files
  (let ((test-names
         '("array-inline-apostrophe"
           "array-inline-bytearray"
           "array-inline-doublequote"
           "array-inline-doublequote-escapes"
           "array-inline-integers"
           "array-inline-nested"
           "array-inline-singlequote"
           "array-multiline"
           "array-multiline-named"
           "array-multiline-nested"
           "array-multiline-nested-multiline-object"
           "array-multiline-triple-nested"
           "boolean-false"
           "boolean-true"
           "bytearray-block-basic"
           "bytearray-block-comment-only"
           "bytearray-block-deeply-nested"
           "bytearray-block-hex-and-comment"
           "bytearray-block-nested-property"
           "bytearray-block-property"
           "bytearray-block-property-comment"
           "bytearray-in-array"
           "bytearray-in-object"
           "bytearray-inline-empty"
           "bytearray-inline-even"
           "bytearray-inline-named"
           "bigint-one"
           "integer-big"
           "integer-big-basic"
           "integer-big-negative"
           "mixed-depth-nesting-1"
           "mixed-depth-nesting-2"
           "mixed-depth-nesting-3"
           "nesting-L0-bytes"
           "nesting-L0-false"
           "nesting-L0-float"
           "nesting-L0-int"
           "nesting-L0-null"
           "nesting-L0-strdq"
           "nesting-L0-strsq"
           "nesting-L0-true"
           "nesting-L1-arr-inline"
           "nesting-L1-arr-multi"
           "nesting-L1-empty-arr"
           "nesting-L1-empty-obj"
           "nesting-L1-named-arr"
           "nesting-L1-obj-inline"
           "nesting-L1-obj-multi"
           "nesting-L2-arr-in-arr-inline"
           "nesting-L2-arr-in-arr-multi"
           "nesting-L2-arr-in-obj-inline"
           "nesting-L2-arr-in-obj-multi"
           "nesting-L2-blockbytes-in-obj"
           "nesting-L2-blockstr-in-obj"
           "nesting-L2-empty-nested"
           "nesting-L2-obj-in-arr-multi"
           "nesting-L2-obj-in-obj-inline"
           "nesting-L2-obj-in-obj-multi"
           "nesting-L3-arr-arr-arr"
           "nesting-L3-arr-arr-arr-inline"
           "nesting-L3-arr-arr-obj"
           "nesting-L3-arr-obj-arr"
           "nesting-L3-arr-obj-obj"
           "nesting-L3-blockbytes-nested"
           "nesting-L3-blockstr-nested"
           "nesting-L3-mixed-inline-in-multi"
           "nesting-L3-obj-arr-arr"
           "nesting-L3-obj-arr-obj"
           "nesting-L3-obj-obj-arr"
           "nesting-L3-obj-obj-obj"
           "null-literal"
           "number-float"
           "number-float-avogadro"
           "number-float-grouped"
           "number-float-infinity"
           "number-float-leading-dot"
           "number-float-nan"
           "number-float-negative-infinity"
           "number-float-negative-zero"
           "number-float-trailing-dot"
           "object-deeply-nested-empty"
           "object-inline-doublequote-key"
           "object-inline-empty"
           "object-inline-integers"
           "object-inline-mixed"
           "object-inline-nested"
           "object-inline-singlequote"
           "object-multiline"
           "object-multiline-doublequote-key"
           "object-multiline-nested"
           "object-multiline-singlequote-key"
           "object-nested-empty-inline"
           "object-nested-empty-property"
           "string-block-deep-indent"
           "string-block-deeply-nested"
           "string-block-empty-middle"
           "string-block-nested-in-object-and-array"
           "string-block-property"
           "string-block-property-empty-middle"
           "string-block-property-trailing-empty"
           "string-block-root-hash"
           "string-block-root-next-line"
           "string-block-root-same-line"
           "string-block-trailing-empty"
           "string-inline-doublequote-apostrophe"
           "string-inline-doublequote-basic"
           "string-inline-doublequote-escaped-quote"
           "string-inline-doublequote-escapes"
           "string-inline-doublequote-space"
           "string-inline-doublequote-unicode-emoji"
           "string-inline-doublequote-unicode-surrogate-pair"
           "string-inline-singlequote-basic"
           "string-inline-singlequote-doublequote"
           "string-multiline-concat"
           "whitespace-leading-lines")))
    (for-each
     (lambda (name)
       (let ((yay-path (string-append test-dir "/yay/" name ".yay"))
             (scm-path (string-append test-dir "/scm/" name ".scm")))
         (catch #t
           (lambda ()
             (let ((yay-content (read-file yay-path))
                   (expected (string-trim-end (read-file scm-path))))
               (add-test! name yay-content expected)))
           (lambda (key . args)
             (display "SKIP: ")
             (display name)
             (display " (file not found)")
             (newline)))))
     test-names)))

;; Load error tests from directory
(define (load-error-tests-from-dir test-dir)
  ;; List of error test base names (without extension)
  ;; These are the tests that have both .nay and .error files
  (let ((error-test-names
         '("array-inline-invalid-multiline"
           "array-inline-invalid-space-after-comma"
           "array-inline-invalid-space-after-open"
           "array-inline-invalid-space-before-close"
           "array-inline-invalid-space-before-comma"
           "array-inline-invalid-tab"
           "array-multiline-invalid-compact"
           "array-multiline-invalid-leading-space"
           "array-multiline-invalid-nested-space"
           "array-multiline-invalid-trailing-space"
           "array-multiline-nested-invalid-bare-word"
           "blank-line-trailing-space"
           "bytearray-block-invalid-empty-leader"
           "bytearray-block-property-invalid-same-line"
           "bytearray-invalid-uppercase-hex"
           "bytearray-inline-invalid-hex-digit"
           "bytearray-inline-invalid-unclosed"
           "bytearray-inline-invalid-unclosed-root"
           "bytearray-inline-odd"
           "character-invalid-asterisk"
           "character-invalid-dollar"
           "comment-only"
           "document-invalid-extra-content"
           "illegal-no-space-after-colon"
           "illegal-no-space-after-comma"
           "illegal-space-after-brace"
           "illegal-space-after-colon"
           "illegal-space-before-angle"
           "illegal-space-before-brace"
           "indent-invalid-unexpected-number"
           "mixed-depth-nesting-2-invalid-bare-word"
           "not-a-string"
           "number-float-invalid-space-after-dot"
           "number-float-invalid-space-before-dot"
           "number-float-invalid-uppercase-exponent"
           "object-inline-invalid-key-symbol"
           "object-inline-invalid-missing-colon"
           "object-inline-invalid-multiline"
           "object-inline-invalid-space-inside"
           "object-inline-invalid-tab"
           "object-invalid-empty-value"
           "object-multiline-invalid-key-char-space"
           "object-multiline-invalid-key-space"
           "object-multiline-invalid-quoted-key-space-after-colon"
           "object-multiline-invalid-quoted-key-space-after-colon-single"
           "object-multiline-invalid-quoted-key-space-before-colon"
           "object-multiline-invalid-quoted-key-space-before-colon-single"
           "object-multiline-invalid-value-space"
           "string-block-invalid-empty"
           "string-block-invalid-tab"
           "string-block-invalid-trailing-space"
           "string-block-property-invalid-same-line"
           "string-inline-doublequote-invalid-escape-x"
           "string-inline-doublequote-invalid-unicode-empty"
           "string-inline-doublequote-invalid-unicode-hex"
           "string-inline-doublequote-invalid-unicode-old-syntax"
           "string-inline-doublequote-invalid-unicode-outofrange"
           "string-inline-doublequote-invalid-unicode-surrogate"
           "string-inline-doublequote-invalid-unicode-toolong"
           "string-inline-invalid-control-char"
           "string-inline-invalid-multiline"
           "unicode-invalid-bom"
           "unicode-invalid-c0-bel"
           "unicode-invalid-c0-bs"
           "unicode-invalid-c0-cr"
           "unicode-invalid-c0-ff"
           "unicode-invalid-c0-nak"
           "unicode-invalid-c0-null"
           "unicode-invalid-c0-so"
           "unicode-invalid-c0-us"
           "unicode-invalid-c0-vt"
           "unicode-invalid-c1-apc"
           "unicode-invalid-c1-ri"
           "unicode-invalid-c1-start"
           "unicode-invalid-del"
           "unicode-invalid-nonchar-1fffe"
           "unicode-invalid-nonchar-1ffff"
           "unicode-invalid-nonchar-fdd0"
           "unicode-invalid-nonchar-fdda"
           "unicode-invalid-nonchar-fdef"
           "unicode-invalid-nonchar-fffe"
           "unicode-invalid-nonchar-ffff"
           "whitespace-invalid-tab"
           "whitespace-invalid-trailing-space")))
    (for-each
     (lambda (name)
       (let ((nay-path (string-append test-dir "/nay/" name ".nay")))
         (catch #t
           (lambda ()
             (let ((nay-content (read-file nay-path)))
               (add-error-test! name nay-content)))
           (lambda (key . args)
             (display "SKIP: ")
             (display name)
             (display " (file not found)")
             (newline)))))
     error-test-names)))

;; Main entry point
(define (main)
  (let ((test-dir "../test"))
    (display "YAY Scheme Parser Test Runner")
    (newline)
    (display "Loading tests from: ")
    (display test-dir)
    (newline)
    (newline)
    
    ;; Load and run valid input tests
    (load-tests-from-dir test-dir)
    (let-values (((passed failed) (run-all-tests)))
      (newline)
      (display "===================")
      (newline)
      (display "Valid input tests: ")
      (display passed)
      (display " passed, ")
      (display failed)
      (display " failed")
      (newline)
      (newline)
      
      ;; Load and run error tests
      (load-error-tests-from-dir test-dir)
      (let-values (((err-passed err-failed) (run-all-error-tests)))
        (newline)
        (display "===================")
        (newline)
        (display "Error tests: ")
        (display err-passed)
        (display " passed, ")
        (display err-failed)
        (display " failed")
        (newline)
        (newline)
        
        ;; Summary
        (display "===================")
        (newline)
        (display "TOTAL: ")
        (display (+ passed err-passed))
        (display " passed, ")
        (display (+ failed err-failed))
        (display " failed")
        (newline)
        
        ;; Exit with error if any tests failed
        (if (> (+ failed err-failed) 0)
            (exit 1)
            (exit 0))))))

;; Run
(main)
