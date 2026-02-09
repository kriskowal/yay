#!/usr/bin/env guile
!#
;;; YAY Test Runner for Scheme
;;; Runs tests using .yay input files and .scm expected output files

(load "yay-parser.scm")

;; Read file contents
(define (read-file-contents path)
  (call-with-input-file path
    (lambda (port)
      (let loop ((chars '()))
        (let ((c (read-char port)))
          (if (eof-object? c)
              (list->string (reverse chars))
              (loop (cons c chars))))))))

;; Get test root directory
(define (get-test-dir)
  (let ((script-dir (dirname (car (command-line)))))
    (if (string=? script-dir ".")
        "../test"
        (string-append script-dir "/../test"))))

;; List files in directory matching pattern
(define (list-test-files dir suffix)
  (let ((entries (scandir dir)))
    (if entries
        (filter (lambda (f) (string-suffix? suffix f))
                entries)
        '())))

;; Check if file exists
(define (file-exists-safe? path)
  (catch #t
    (lambda () (access? path R_OK))
    (lambda args #f)))

;; Run a single test
(define (run-test test-dir base-name)
  (let* ((yay-file (string-append test-dir "/yay/" base-name ".yay"))
         (scm-file (string-append test-dir "/scm/" base-name ".scm")))
    (if (and (file-exists-safe? yay-file)
             (file-exists-safe? scm-file))
        (catch #t
          (lambda ()
            (let* ((yay-source (read-file-contents yay-file))
                   (expected-raw (read-file-contents scm-file))
                   (expected (string-trim-right expected-raw))
                   (parsed (parse-yay yay-source))
                   (actual (yay->scheme-string parsed)))
              (if (string=? actual expected)
                  (begin
                    (display "PASS: ")
                    (display base-name)
                    (newline)
                    #t)
                  (begin
                    (display "FAIL: ")
                    (display base-name)
                    (newline)
                    (display "  Expected: ")
                    (display expected)
                    (newline)
                    (display "  Actual:   ")
                    (display actual)
                    (newline)
                    #f))))
          (lambda (key . args)
            (display "ERROR: ")
            (display base-name)
            (display " - ")
            (display key)
            (display ": ")
            (display args)
            (newline)
            #f))
        (begin
          (display "SKIP: ")
          (display base-name)
          (display " (missing files)")
          (newline)
          #t))))

;; String trim right
(define (string-trim-right s)
  (let loop ((i (string-length s)))
    (if (or (<= i 0)
            (not (or (char=? (string-ref s (- i 1)) #\newline)
                     (char=? (string-ref s (- i 1)) #\space)
                     (char=? (string-ref s (- i 1)) #\return))))
        (substring s 0 i)
        (loop (- i 1)))))

;; String suffix check
(define (string-suffix? suffix s)
  (let ((slen (string-length s))
        (suflen (string-length suffix)))
    (and (>= slen suflen)
         (string=? (substring s (- slen suflen) slen) suffix))))

;; Main
(define (main args)
  (let* ((test-dir (if (and (pair? (cdr args))
                            (not (string-prefix? "-" (cadr args))))
                       (cadr args)
                       (get-test-dir)))
         (scm-files (list-test-files (string-append test-dir "/scm") ".scm"))
         (base-names (map (lambda (f) 
                            (substring f 0 (- (string-length f) 4)))
                          scm-files)))
    (display "Running YAY Scheme tests from: ")
    (display test-dir)
    (newline)
    (display "Found ")
    (display (length base-names))
    (display " tests")
    (newline)
    (newline)
    (let loop ((tests base-names) (passed 0) (failed 0))
      (if (null? tests)
          (begin
            (newline)
            (display "Results: ")
            (display passed)
            (display " passed, ")
            (display failed)
            (display " failed")
            (newline)
            (if (> failed 0) 1 0))
          (let ((result (run-test test-dir (car tests))))
            (loop (cdr tests)
                  (if result (+ passed 1) passed)
                  (if result failed (+ failed 1))))))))

;; String prefix check
(define (string-prefix? prefix s)
  (let ((slen (string-length s))
        (prelen (string-length prefix)))
    (and (>= slen prelen)
         (string=? (substring s 0 prelen) prefix))))

;; Run main if executed directly
(main (command-line))
