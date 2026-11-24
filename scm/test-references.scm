;;; Reference tests for Scheme (Guile)
;;; Usage: guile --no-auto-compile test-references.scm
;;;
;;; MEH's leniency is not YAY's, so the corpus's MEH reference cases,
;;; ../test/meh/reference-*.meh, are refused as YAY. The corpus reference
;;; cases in test/yay and test/nay are read with the rest of the corpus.

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

;; The message a source is refused with, or #f
(define (refusal source)
  (catch #t
    (lambda () (parse-yay source) #f)
    (lambda (key . args)
      (if (and (pair? args) (pair? (cdr args)) (pair? (cddr args))
               (pair? (caddr args)))
          (car (caddr args))
          (format #f "~a" args)))))

(define (has-ext? name ext) (string-suffix? ext name))

(define (test-meh-cases)
  (let ((names (filter (lambda (name)
                         (and (string-prefix? "reference-" name)
                              (has-ext? name ".meh")))
                       (scandir meh-dir))))
    (check (>= (length names) 2) "MEH reference cases read")
    (for-each
     (lambda (name)
       (check (refused? (read-file (string-append meh-dir "/" name)))
              (string-append name ": accepted, should be refused")))
     names)))

(define (test-model)
  (let ((a (make-reference "https://x")))
    (check (reference? a) "reference?")
    (check (string=? (reference-uri a) "https://x") "reference-uri")
    (check (yay-equal? a (make-reference "https://x")) "equal by text")
    (check (not (yay-equal? a (make-reference "https://x/"))) "texts differ")
    (check (not (yay-equal? (make-reference "a:%2f") (make-reference "a:%2F")))
           "percent-encodings are not normalized")
    (check (and (not (yay-equal? a "https://x")) (not (yay-equal? "https://x" a)))
           "never a string")
    (check (not (yay-equal? a (make-tagged "https://x" 'null))) "never a tagged value")
    (check (string=? (yay->scheme-string a) "(make-reference \"https://x\")") "printed")))

;; The Rust, JavaScript, Python and Go ports' messages (this port reports no
;; columns)
(define (test-refused)
  (for-each
   (lambda (c)
     (let ((got (refusal (car c))))
       (check (equal? got (cadr c))
              (string-append (car c) ": got " (if got got "a value")
                             ", want " (cadr c)))))
   '(("a: <HTTPS://x>\n" "Uppercase scheme \"HTTPS\" in reference (use lowercase)")
     ("a: <https://x y>\n" "Unexpected space in reference")
     ("a: <https://x\"y>\n" "Unexpected character \"\"\" in reference")
     ("a: <https://x/%2g>\n" "Bad percent-encoding in reference (use % and two hex digits)")
     ("a: <https://x/%2>\n" "Bad percent-encoding in reference (use % and two hex digits)")
     ("a: <https://x\n" "Unmatched angle bracket")
     ("a: <https://x>(t)\n" "Expected one space before a tag")
     ("a: <https://x>rgb\n" "Unexpected character \"r\"")
     ("<https://x>: 1\n" "Unexpected character \":\""))))

(define (test-parse)
  (check (yay-equal? (parse-yay "<ocapn://a.tcp> (Counter)\n")
                     (make-tagged "Counter" (make-reference "ocapn://a.tcp")))
         "a tagged reference at the root")
  ;; Unchanged in strings and tags, and bytes without a scheme
  (check (yay-equal? (parse-yay "a: '<https://x>' (<b:c>)\nb: <ff>\n")
                     (list (cons "a" (make-tagged "<b:c>" "<https://x>"))
                           (cons "b" (parse-hex-bytes "ff"))))
         "strings, tags and bytes")
  ;; A reference's punctuation is its own
  (check (yay-equal? (parse-yay "a: [<x:a,b]c#d>, 1]  # c\n")
                     (list (cons "a" (vector (make-reference "x:a,b]c#d") 1))))
         "punctuation in an inline array")
  (check (yay-equal? (parse-yay "a: {b: <x:(t)>} (u)\n")
                     (list (cons "a" (make-tagged "u" (list (cons "b" (make-reference "x:(t)")))))))
         "punctuation in an inline object"))

;; A reference is made only from a reference's text
(define (test-constructor)
  (for-each
   (lambda (uri)
     (let ((got (catch #t
                  (lambda () (make-reference uri) #f)
                  (lambda (key . args)
                    (if (and (pair? args) (pair? (cdr args)) (pair? (cddr args))
                             (pair? (caddr args)))
                        (car (caddr args))
                        (format #f "~a" args))))))
       (check (and got (string-prefix? "Not a reference: " got))
              (string-append "make-reference " (format #f "~s" uri) ": "
                             (if got got "made")))))
   '("" "not a uri" "HTTPS://x" "urn:a b" "https://a>b" "https://x/%zz" "./a.yay"))
  (for-each
   (lambda (uri)
     (check (and (string=? (reference-uri (make-reference uri)) uri)
                 (yay-equal? (parse-yay (string-append "<" uri ">\n"))
                             (make-reference uri)))
            (string-append "make-reference " uri)))
   (list "urn:x" "https://example.com/a?b=c#d" "urn:%C3%A9"
         (string-append "urn:" (string (integer->char #xE9))))))

(test-constructor)
(test-model)
(test-refused)
(test-parse)
(test-meh-cases)
(display "references: ")
(display passed)
(display " passed, ")
(display failed)
(display " failed")
(newline)
(exit (if (= failed 0) 0 1))
