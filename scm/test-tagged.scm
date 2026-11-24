;;; Tagged value tests for Scheme (Guile)
;;; Usage: guile --no-auto-compile test-tagged.scm
;;;
;;; MEH's leniency is not YAY's, so the corpus's MEH tagged cases,
;;; ../test/meh/tagged-*.meh, are refused as YAY. The corpus tagged cases in
;;; test/yay and test/nay are read with the rest of the corpus.

(use-modules (ice-9 ftw) (ice-9 textual-ports))

(load "yay-parser.scm")

(define meh-dir "../test/meh")
(define passed 0)
(define failed 0)

(define (check ok what)
  (if ok
      (set! passed (+ passed 1))
      (begin
        (set! failed (+ failed 1))
        (display "FAIL: ")
        (display what)
        (newline))))

(define (read-file path)
  (call-with-input-file path get-string-all #:encoding "UTF-8"))

(define (refused? source)
  (catch #t
    (lambda () (parse-yay source) #f)
    (lambda args #t)))

(define (has-ext? name ext) (string-suffix? ext name))

(define (test-meh-cases)
  (let ((names (filter (lambda (name)
                         (and (string-prefix? "tagged-" name)
                              (has-ext? name ".meh")))
                       (scandir meh-dir))))
    (check (>= (length names) 3) "MEH tagged cases read")
    (for-each
     (lambda (name)
       (check (refused? (read-file (string-append meh-dir "/" name)))
              (string-append name ": accepted, should be refused")))
     names)))

(define (test-model)
  (let ((a (make-tagged "t" 1)))
    (check (tagged? a) "tagged?")
    (check (string=? (tagged-tag a) "t") "tagged-tag")
    (check (= (tagged-payload a) 1) "tagged-payload")
    (check (yay-equal? a (make-tagged "t" 1)) "equal tags and payloads")
    (check (not (yay-equal? a (make-tagged "u" 1))) "tags differ")
    (check (not (yay-equal? a (make-tagged "t" 2))) "payloads differ")
    (check (and (not (yay-equal? a 1)) (not (yay-equal? 1 a))) "never its payload")
    (check (string=? (yay->scheme-string a) "(make-tagged \"t\" 1)") "printed")))

(define (test-escapes-and-nesting)
  (check (yay-equal? (parse-yay "a: 1 (x\\u{41}\\\\\\n) (outer)\n")
                     (list (cons "a" (make-tagged "outer" (make-tagged "xA\\\n" 1)))))
         "escapes and nesting innermost first")
  (check (yay-equal? (parse-yay "- (a)\n  - (b)\n    k: 'v'\n")
                     (vector (make-tagged "a" (vector (make-tagged "b" (list (cons "k" "v")))))))
         "tagged blocks within tagged blocks")
  (check (yay-equal? (parse-yay "a: 1 (t)  # note\n")
                     (list (cons "a" (make-tagged "t" 1))))
         "a comment after a tag")
  (for-each
   (lambda (source)
     (check (refused? source) (string-append "refused: " source)))
   '("a: 1 (x\\\"y)\n" "a: 1 (x\\(y)\n" "a: 1 (x\n" "a: 1  (t)\n" "a: 1 (t)  b\n"
     "a: (t)\n" "- (t) 1\n" "(t)\n1\n" "a: [1(t)]\n" "a: {b: 'x'(t)}\n")))

(test-model)
(test-escapes-and-nesting)
(test-meh-cases)
(display "tagged: ")
(display passed)
(display " passed, ")
(display failed)
(display " failed")
(newline)
(exit (if (= failed 0) 0 1))
