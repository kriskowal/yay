;;; YAY Parser for Scheme
;;; A standalone parser that works with Guile, Chez, Chicken, etc.

(use-modules (ice-9 receive))

;; Error handling
(define (yay-error msg)
  (error msg))

;; Character predicates
(define (digit? c)
  (and (char? c) (char>=? c #\0) (char<=? c #\9)))

(define (alpha? c)
  (and (char? c)
       (or (and (char>=? c #\a) (char<=? c #\z))
           (and (char>=? c #\A) (char<=? c #\Z)))))

(define (hex-digit? c)
  (and (char? c)
       (or (digit? c)
           (and (char>=? c #\a) (char<=? c #\f))
           (and (char>=? c #\A) (char<=? c #\F)))))

(define (uppercase-hex? c)
  (and (char? c)
       (char>=? c #\A)
       (char<=? c #\F)))

(define (hex-value c)
  (cond ((digit? c) (- (char->integer c) (char->integer #\0)))
        ((and (char>=? c #\a) (char<=? c #\f))
         (+ 10 (- (char->integer c) (char->integer #\a))))
        ((and (char>=? c #\A) (char<=? c #\F))
         (+ 10 (- (char->integer c) (char->integer #\A))))
        (else 0)))

;; String utilities
(define (string-starts-with? s prefix)
  (and (>= (string-length s) (string-length prefix))
       (string=? (substring s 0 (string-length prefix)) prefix)))

(define (string-ends-with? s suffix)
  (let ((slen (string-length s))
        (suflen (string-length suffix)))
    (and (>= slen suflen)
         (string=? (substring s (- slen suflen) slen) suffix))))

(define (string-index s c)
  (let loop ((i 0))
    (cond ((>= i (string-length s)) #f)
          ((char=? (string-ref s i) c) i)
          (else (loop (+ i 1))))))

(define (string-contains? s sub)
  (let ((slen (string-length s))
        (sublen (string-length sub)))
    (let loop ((i 0))
      (cond ((> (+ i sublen) slen) #f)
            ((string=? (substring s i (+ i sublen)) sub) #t)
            (else (loop (+ i 1)))))))

(define (string-split-newlines s)
  (let ((len (string-length s)))
    (let loop ((start 0) (i 0) (result '()))
      (cond ((>= i len)
             (reverse (cons (substring s start len) result)))
            ((char=? (string-ref s i) #\newline)
             (loop (+ i 1) (+ i 1) (cons (substring s start i) result)))
            (else (loop start (+ i 1) result))))))

(define (string-join lst sep)
  (if (null? lst)
      ""
      (let loop ((rest (cdr lst)) (result (car lst)))
        (if (null? rest)
            result
            (loop (cdr rest) (string-append result sep (car rest)))))))

(define (string-trim s)
  (let* ((len (string-length s))
         (start (let loop ((i 0))
                  (if (or (>= i len) 
                          (not (or (char=? (string-ref s i) #\space)
                                   (char=? (string-ref s i) #\tab))))
                      i
                      (loop (+ i 1)))))
         (end (let loop ((i len))
                (if (or (<= i start) 
                        (not (or (char=? (string-ref s (- i 1)) #\space)
                                 (char=? (string-ref s (- i 1)) #\tab))))
                    i
                    (loop (- i 1))))))
    (substring s start end)))

;; Count leading spaces
(define (count-leading-spaces s)
  (let loop ((i 0))
    (if (or (>= i (string-length s))
            (not (char=? (string-ref s i) #\space)))
        i
        (loop (+ i 1)))))

;; Check for NaN (portable)
(define (my-nan? x)
  (and (number? x) (inexact? x) (not (= x x))))

;; Check for infinity (portable)
(define (my-infinite? x)
  (and (number? x) (inexact? x) (= x x) (not (finite? x))))

;; Check if finite (for older Schemes that might not have it)
(define (finite? x)
  (and (not (my-nan? x))
       (< (abs x) +inf.0)))

;; Check whether a code point is allowed in a YAY document.
(define (allowed-code-point? cp)
  (or (= cp #x000A)
      (and (>= cp #x0020) (<= cp #x007E))
      (and (>= cp #x00A0) (<= cp #xD7FF))
      (and (>= cp #xE000) (<= cp #xFFFD)
           (not (and (>= cp #xFDD0) (<= cp #xFDEF))))
      (and (>= cp #x10000) (<= cp #x10FFFF)
           (< (logand cp #xFFFF) #xFFFE))))

;; Scanner: source -> list of (line indent leader linenum)
(define (scan source)
  ;; Check for BOM
  (when (and (> (string-length source) 0)
             (char=? (string-ref source 0) #\xFEFF))
    (yay-error "Illegal BOM"))
  ;; Check for forbidden code points
  (let check-cp ((i 0))
    (when (< i (string-length source))
      (let ((cp (char->integer (string-ref source i))))
        (when (not (allowed-code-point? cp))
          (cond
           ((= cp 9)
            (yay-error "Tab not allowed (use spaces)"))
           ((and (>= cp #xD800) (<= cp #xDFFF))
            (yay-error "Illegal surrogate"))
           (else
            (yay-error (format #f "Forbidden code point U+~4,'0X" cp)))))
        (check-cp (+ i 1)))))
  ;; Split into lines
  (let ((lines-raw (string-split-newlines source)))
    (let loop ((lines lines-raw) (linenum 0) (acc '()))
      (if (null? lines)
          (reverse acc)
          (let* ((line-str (car lines))
                 (len (string-length line-str)))
            ;; Check for trailing space
            (when (and (> len 0)
                       (char=? (string-ref line-str (- len 1)) #\space))
              (yay-error "Unexpected trailing space"))
            ;; Count indent
            (let* ((indent (count-leading-spaces line-str))
                   (rest (if (>= indent len) "" (substring line-str indent len))))
              ;; Skip comment-only lines at column 0
              (if (and (string-starts-with? rest "#") (= indent 0))
                  (loop (cdr lines) (+ linenum 1) acc)
                  ;; Determine leader
                  (let ((leader-and-content
                         (cond
                          ;; "- " prefix - check for double space error
                          ((string-starts-with? rest "- ")
                           (let ((content (substring rest 2 (string-length rest))))
                             ;; Check if content starts with "- " followed by space (nested array with extra space)
                             (when (and (>= (string-length content) 2)
                                        (char=? (string-ref content 0) #\-)
                                        (char=? (string-ref content 1) #\space)
                                        (>= (string-length content) 3)
                                        (char=? (string-ref content 2) #\space))
                               (yay-error "Unexpected space after \"-\""))
                             (cons "-" content)))
                          ;; Bare "-"
                          ((string=? rest "-")
                           (cons "-" ""))
                          ;; "-infinity" keyword
                          ((string=? rest "-infinity")
                           (cons "" rest))
                          ;; "-" followed by digit or dot (negative number)
                          ((and (> (string-length rest) 1)
                                (char=? (string-ref rest 0) #\-)
                                (or (digit? (string-ref rest 1))
                                    (char=? (string-ref rest 1) #\.)))
                           (cons "" rest))
                          ;; "-" followed by non-space, non-dot, non-digit is an error
                          ;; (compact array syntax like "-a" is not allowed)
                          ((and (>= (string-length rest) 2)
                                (char=? (string-ref rest 0) #\-)
                                (not (char=? (string-ref rest 1) #\space))
                                (not (char=? (string-ref rest 1) #\.))
                                (not (digit? (string-ref rest 1))))
                           (yay-error "Expected space after \"-\""))
                          ;; Asterisk alone or with space (error in YAY)
                          ((or (string=? rest "*")
                               (and (>= (string-length rest) 2)
                                    (char=? (string-ref rest 0) #\*)
                                    (char=? (string-ref rest 1) #\space)))
                           (yay-error "Unexpected character \"*\""))
                          (else (cons "" rest)))))
                    (loop (cdr lines) (+ linenum 1)
                          (cons (list (cdr leader-and-content) 
                                      indent 
                                      (car leader-and-content) 
                                      linenum) 
                                acc))))))))))

;; Tokenizer: scanned lines -> tokens
(define (tokenize lines)
  (let ((tokens '())
        (stack (list 0))
        (broken #f)
        (first-line #t))
    (for-each
     (lambda (entry)
       (let* ((line (list-ref entry 0))
              (indent (list-ref entry 1))
              (leader (list-ref entry 2))
              (linenum (list-ref entry 3))
              (top (car stack)))
         ;; Check for unexpected indent at root level
         (when (and first-line (> indent 0) (> (string-length line) 0))
           (yay-error "Unexpected indent"))
         (when (> (string-length line) 0)
           (set! first-line #f))
         ;; Close blocks on dedent
         (let close-loop ()
           (when (< indent (car stack))
             (set! tokens (cons (list 'stop "") tokens))
             (set! stack (cdr stack))
             (close-loop)))
         (let ((top (car stack)))
           ;; Handle leader
           (cond
            ((and (> (string-length leader) 0) (> indent top))
             (set! tokens (cons (list 'start leader indent linenum) tokens))
             (set! stack (cons indent stack))
             (set! broken #f))
            ((and (> (string-length leader) 0) (= indent top))
             (set! tokens (cons (list 'stop "") tokens))
             (set! tokens (cons (list 'start leader indent linenum) tokens))
             (set! broken #f)))
           ;; Handle line content
           (cond
            ((> (string-length line) 0)
             (set! tokens (cons (list 'text line indent linenum) tokens))
             (set! broken #f))
            ((not broken)
             (set! tokens (cons (list 'break "" linenum) tokens))
             (set! broken #t))))))
     lines)
    ;; Close remaining blocks
    (let close-final ()
      (when (> (length stack) 1)
        (set! tokens (cons (list 'stop "") tokens))
        (set! stack (cdr stack))
        (close-final)))
    (reverse tokens)))

;; Token accessors
(define (token-type t) (list-ref t 0))
(define (token-text t) (list-ref t 1))
(define (token-indent t) (if (>= (length t) 3) (list-ref t 2) 0))
(define (token-linenum t) (if (>= (length t) 4) (list-ref t 3) 0))

;; Validate no spaces around decimal point
(define (validate-number-spaces s)
  (let ((len (string-length s)))
    (let loop ((i 0))
      (when (< i len)
        (let ((c (string-ref s i)))
          (when (char=? c #\.)
            ;; Check for space before dot
            (when (and (> i 0) (char=? (string-ref s (- i 1)) #\space))
              (yay-error "Unexpected space in number"))
            ;; Check for space after dot
            (when (and (< (+ i 1) len) (char=? (string-ref s (+ i 1)) #\space))
              (yay-error "Unexpected space in number")))
          (loop (+ i 1)))))))

;; Parse number (returns #f if not a number)
(define (parse-number s)
  ;; First validate no spaces around decimal point
  (validate-number-spaces s)
  ;; Check for uppercase E in exponent (only if string looks like a number)
  (let ((first-char (if (> (string-length s) 0) (string-ref s 0) #\space)))
    (when (or (digit? first-char) (char=? first-char #\-) (char=? first-char #\.))
      (let ((e-idx (string-index s #\E)))
        (when e-idx
          (yay-error "Uppercase exponent (use lowercase 'e')")))))
  (let* ((compact (let loop ((i 0) (acc '()))
                    (if (>= i (string-length s))
                        (list->string (reverse acc))
                        (let ((c (string-ref s i)))
                          (if (char=? c #\space)
                              (loop (+ i 1) acc)
                              (loop (+ i 1) (cons c acc)))))))
         (len (string-length compact))
         ;; Check for exponent character in compact string
         (has-exp-char (string-index compact #\e)))
    (cond
     ((= len 0) #f)
     ;; Check if it's a valid number pattern (with exponent support)
     ((let ((has-dot #f)
            (has-digit #f)
            (has-exponent #f))
        (let loop ((i 0))
          (if (>= i len)
              has-digit
              (let ((c (string-ref compact i)))
                (cond
                 ((and (= i 0) (char=? c #\-))
                  (loop (+ i 1)))
                 ((char=? c #\.)
                  (if (or has-dot has-exponent)
                      #f
                      (begin (set! has-dot #t) (loop (+ i 1)))))
                 ;; Exponent notation (lowercase only - uppercase already rejected)
                 ((and (char=? c #\e) has-digit (not has-exponent))
                  (set! has-exponent #t)
                  (loop (+ i 1)))
                 ;; Allow +/- after exponent
                 ((and (or (char=? c #\+) (char=? c #\-)) has-exponent (> i 0))
                  (let ((prev (string-ref compact (- i 1))))
                    (if (char=? prev #\e)
                        (loop (+ i 1))
                        #f)))
                 ((digit? c)
                  (set! has-digit #t)
                  (loop (+ i 1)))
                 (else #f))))))
      ;; It's a valid number
      (if (or (string-index compact #\.) has-exp-char)
          ;; Float (has decimal point or exponent)
          (string->number compact)
          ;; Integer
          (string->number compact)))
     (else #f))))

;; Parse JSON-style quoted string
(define (parse-json-string s)
  (if (or (< (string-length s) 2)
          (not (char=? (string-ref s 0) #\")))
      s
      (let ((len (string-length s)))
        (when (not (char=? (string-ref s (- len 1)) #\"))
          (yay-error "Unterminated string"))
        (let loop ((i 1) (acc '()))
          (if (>= i (- len 1))
              (list->string (reverse acc))
              (let ((ch (string-ref s i)))
                (if (char=? ch #\\)
                    ;; Escape sequence
                    (if (>= (+ i 1) (- len 1))
                        (yay-error "Bad escaped character")
                        (let ((esc (string-ref s (+ i 1))))
                          (case esc
                            ((#\") (loop (+ i 2) (cons #\" acc)))
                            ((#\\) (loop (+ i 2) (cons #\\ acc)))
                            ((#\/) (loop (+ i 2) (cons #\/ acc)))
                            ((#\b) (loop (+ i 2) (cons #\backspace acc)))
                            ((#\f) (loop (+ i 2) (cons (integer->char 12) acc)))
                            ((#\n) (loop (+ i 2) (cons #\newline acc)))
                            ((#\r) (loop (+ i 2) (cons #\return acc)))
                            ((#\t) (loop (+ i 2) (cons #\tab acc)))
                            ((#\u)
                             ;; Expect \u{XXXXXX} format
                             (if (and (< (+ i 2) len)
                                      (char=? (string-ref s (+ i 2)) #\{))
                                 ;; Find closing brace
                                 (let find-brace ((j (+ i 3)))
                                   (if (>= j len)
                                       (yay-error "Bad Unicode escape")
                                       (if (char=? (string-ref s j) #\})
                                           (let* ((hex-start (+ i 3))
                                                  (hex-len (- j hex-start))
                                                  (hex-str (substring s hex-start j)))
                                             (if (or (= hex-len 0) (> hex-len 6))
                                                 (yay-error "Bad Unicode escape")
                                                 (let ((valid (let check ((k 0))
                                                                (or (>= k hex-len)
                                                                    (and (hex-digit? (string-ref hex-str k))
                                                                         (check (+ k 1)))))))
                                                   (if (not valid)
                                                       (yay-error "Bad Unicode escape")
                                                       (let ((code (string->number hex-str 16)))
                                                         (cond
                                                          ((and (>= code #xD800) (<= code #xDFFF))
                                                           (yay-error "Illegal surrogate"))
                                                          ((> code #x10FFFF)
                                                           (yay-error "Bad Unicode escape"))
                                                          (else
                                                           (loop (+ j 1) (cons (integer->char code) acc)))))))))
                                           (find-brace (+ j 1)))))
                                 (yay-error "Bad Unicode escape")))
                            (else (yay-error "Bad escaped character")))))
                    ;; Regular character
                    (begin
                      (when (< (char->integer ch) #x20)
                        (yay-error "Bad character in string"))
                      (loop (+ i 1) (cons ch acc))))))))))

;; Parse single-quoted string
(define (parse-single-quoted-string s)
  (if (or (< (string-length s) 2)
          (not (char=? (string-ref s 0) #\')))
      s
      (let* ((len (string-length s))
             (inner (substring s 1 (- len 1))))
        ;; Handle \' escapes
        (let loop ((i 0) (acc '()))
          (if (>= i (string-length inner))
              (list->string (reverse acc))
              (let ((ch (string-ref inner i)))
                (if (and (char=? ch #\\)
                         (< (+ i 1) (string-length inner))
                         (char=? (string-ref inner (+ i 1)) #\'))
                    (loop (+ i 2) (cons #\' acc))
                    (loop (+ i 1) (cons ch acc)))))))))

;; Parse hex bytes - returns a tagged bytevector
(define (parse-hex-bytes hex-str)
  (let* ((hex (let loop ((i 0) (acc '()))
                (if (>= i (string-length hex-str))
                    (list->string (reverse acc))
                    (let ((c (string-ref hex-str i)))
                      (if (or (char=? c #\space) (char=? c #\newline))
                          (loop (+ i 1) acc)
                          (begin
                            ;; Check for uppercase hex digit
                            (when (uppercase-hex? c)
                              (yay-error "Uppercase hex digit (use lowercase)"))
                            ;; Validate hex digit
                            (when (not (hex-digit? c))
                              (yay-error "Invalid hex digit"))
                            (loop (+ i 1) (cons c acc))))))))
         (len (string-length hex)))
    (when (odd? len)
      (yay-error "Odd number of hex digits in byte literal"))
    (let loop ((i 0) (bytes '()))
      (if (>= i len)
          ;; Tag as bytevector
          (cons 'bytevector (reverse bytes))
          (let ((hi (hex-value (string-ref hex i)))
                (lo (hex-value (string-ref hex (+ i 1)))))
            (loop (+ i 2) (cons (+ (* hi 16) lo) bytes)))))))

;; Parse angle-bracket bytes <...>
(define (parse-angle-bytes s)
  (let ((len (string-length s)))
    ;; Validate: no space after <
    (when (and (> len 1) (char=? (string-ref s 1) #\space))
      (yay-error "Unexpected space after \"<\""))
    ;; Validate: no space before >
    (when (and (> len 1) (char=? (string-ref s (- len 2)) #\space))
      (yay-error "Unexpected space before \">\""))
    (if (string=? s "<>")
        (cons 'bytevector '())  ;; Empty bytevector
        (let* ((inner (substring s 1 (- len 1))))
          ;; Check for uppercase hex digits
          (let loop ((i 0))
            (when (< i (string-length inner))
              (let ((c (string-ref inner i)))
                (when (uppercase-hex? c)
                  (yay-error "Uppercase hex digit (use lowercase)"))
                (loop (+ i 1)))))
          (parse-hex-bytes inner)))))

;; Validate inline syntax (whitespace rules)
(define (validate-inline-syntax s)
  (let ((len (string-length s)))
    ;; Check for tabs
    (when (string-index s #\tab)
      (yay-error "Unexpected tab character"))
    ;; Check for space after opening bracket/brace/angle
    (when (and (> len 1)
               (or (char=? (string-ref s 0) #\[)
                   (char=? (string-ref s 0) #\{)
                   (char=? (string-ref s 0) #\<))
               (char=? (string-ref s 1) #\space))
      (yay-error (string-append "Unexpected space after \"" (string (string-ref s 0)) "\"")))
    ;; Check for space before closing bracket/brace/angle
    (when (and (> len 1)
               (or (char=? (string-ref s (- len 1)) #\])
                   (char=? (string-ref s (- len 1)) #\})
                   (char=? (string-ref s (- len 1)) #\>))
               (char=? (string-ref s (- len 2)) #\space))
      (yay-error (string-append "Unexpected space before \"" (string (string-ref s (- len 1))) "\"")))
    ;; Check comma spacing (no space before comma, exactly one space after)
    (let loop ((i 0) (in-string #f) (string-char #f))
      (when (< i len)
        (let ((ch (string-ref s i)))
          (cond
           ;; Track string state
           ((and (not in-string) (or (char=? ch #\") (char=? ch #\')))
            (loop (+ i 1) #t ch))
           ((and in-string (char=? ch string-char) 
                 (or (= i 0) (not (char=? (string-ref s (- i 1)) #\\))))
            (loop (+ i 1) #f #f))
           ;; Check comma rules (only outside strings)
           ((and (not in-string) (char=? ch #\,))
            ;; Space before comma is error
            (when (and (> i 0) (char=? (string-ref s (- i 1)) #\space))
              (yay-error "Unexpected space before \",\""))
            ;; Check space after comma
            (when (< (+ i 1) len)
              (let ((next-ch (string-ref s (+ i 1))))
                ;; No space after comma (unless closing bracket)
                (when (and (not (char=? next-ch #\space))
                           (not (char=? next-ch #\]))
                           (not (char=? next-ch #\})))
                  (yay-error "Expected space after \",\""))
                ;; Double space after comma
                (when (and (char=? next-ch #\space)
                           (< (+ i 2) len)
                           (char=? (string-ref s (+ i 2)) #\space))
                  (yay-error "Unexpected space after \",\""))))
            (loop (+ i 1) in-string string-char))
           (else
            (loop (+ i 1) in-string string-char))))))))

;; Parse inline array [...]
(define (parse-inline-array s)
  (let ((len (string-length s)))
    (when (not (string-contains? s "]"))
      (yay-error "Unexpected newline in inline array"))
    (validate-inline-syntax s)
    ;; Simple parser for inline arrays
    (let loop ((i 1) (items '()) (current ""))
      (if (>= i len)
          (list->vector (reverse items))
          (let ((ch (string-ref s i)))
            (cond
             ;; End of array
             ((char=? ch #\])
              (let ((items (if (string=? (string-trim current) "")
                               items
                               (cons (parse-inline-value (string-trim current)) items))))
                (list->vector (reverse items))))
             ;; Comma separator - skip following space
             ((char=? ch #\,)
              (let ((trimmed (string-trim current)))
                (let ((new-items (if (string=? trimmed "")
                                     items
                                     (cons (parse-inline-value trimmed) items)))
                      ;; Skip space after comma
                      (next-i (if (and (< (+ i 1) len)
                                       (char=? (string-ref s (+ i 1)) #\space))
                                  (+ i 2)
                                  (+ i 1))))
                  (loop next-i new-items ""))))
             ;; Start of nested array
             ((char=? ch #\[)
              (receive (nested end-i) (parse-nested-inline-array s i)
                (loop end-i (cons nested items) "")))
             ;; Start of string
             ((or (char=? ch #\") (char=? ch #\'))
              (receive (str end-i) (parse-inline-string s i ch)
                (loop end-i items (string-append current str))))
             ;; Start of bytes
             ((char=? ch #\<)
              (receive (bytes end-i) (parse-inline-bytes s i)
                (loop end-i (cons bytes items) "")))
             ;; Regular character
             (else
              (loop (+ i 1) items (string-append current (string ch))))))))))

;; Parse nested inline array
(define (parse-nested-inline-array s start)
  (let loop ((i (+ start 1)) (depth 1) (content "["))
    (if (or (>= i (string-length s)) (= depth 0))
        (values (parse-inline-array content) i)
        (let ((ch (string-ref s i)))
          (cond
           ((char=? ch #\[)
            (loop (+ i 1) (+ depth 1) (string-append content (string ch))))
           ((char=? ch #\])
            (if (= depth 1)
                (values (parse-inline-array (string-append content "]")) (+ i 1))
                (loop (+ i 1) (- depth 1) (string-append content (string ch)))))
           ((or (char=? ch #\") (char=? ch #\'))
            (receive (str end-i) (parse-inline-string s i ch)
              (loop end-i depth (string-append content str))))
           (else
            (loop (+ i 1) depth (string-append content (string ch)))))))))

;; Parse inline string within array
(define (parse-inline-string s start quote)
  (let loop ((i (+ start 1)) (content (string quote)) (escaped #f))
    (if (>= i (string-length s))
        (values content i)
        (let ((ch (string-ref s i)))
          (cond
           (escaped
            (loop (+ i 1) (string-append content (string ch)) #f))
           ((char=? ch #\\)
            (loop (+ i 1) (string-append content (string ch)) #t))
           ((char=? ch quote)
            (values (string-append content (string ch)) (+ i 1)))
           (else
            (loop (+ i 1) (string-append content (string ch)) #f)))))))

;; Parse inline bytes within array
(define (parse-inline-bytes s start)
  (let loop ((i (+ start 1)) (content ""))
    (if (>= i (string-length s))
        (values (parse-hex-bytes content) i)
        (let ((ch (string-ref s i)))
          (if (char=? ch #\>)
              (values (parse-hex-bytes content) (+ i 1))
              (loop (+ i 1) (string-append content (string ch))))))))

;; Parse inline object {...}
(define (parse-inline-object s)
  (let ((len (string-length s)))
    (when (not (string-contains? s "}"))
      (yay-error "Unexpected newline in inline object"))
    (validate-inline-syntax s)
    ;; Simple parser for inline objects
    (let loop ((i 1) (pairs '()) (current-key #f) (current ""))
      (if (>= i len)
          (reverse pairs)
          (let ((ch (string-ref s i)))
            (cond
             ;; End of object
             ((char=? ch #\})
              (let ((trimmed-current (if (string? current) (string-trim current) "")))
                ;; If we have content but no key, we're missing a colon
                (when (and (not current-key) (> (string-length trimmed-current) 0))
                  (yay-error "Expected colon after key"))
                (let ((pairs (if current-key
                                 (cons (cons current-key 
                                             (if (string? current)
                                                 (parse-inline-value trimmed-current)
                                                 current))  ;; Already parsed (nested object/array)
                                       pairs)
                                 pairs)))
                  (reverse pairs))))
             ;; Colon separator - we have the key
             ((and (not current-key) (char=? ch #\:))
              (let ((key (string-trim current)))
                ;; Parse key (may be quoted)
                (let ((parsed-key (cond
                                   ((string-starts-with? key "\"") (parse-json-string key))
                                   ((string-starts-with? key "'") (parse-single-quoted-string key))
                                   (else
                                    ;; Validate unquoted key
                                    (validate-unquoted-key-inline key)
                                    key))))
                  ;; Skip space after colon
                  (let ((next-i (if (and (< (+ i 1) len)
                                         (char=? (string-ref s (+ i 1)) #\space))
                                    (+ i 2)
                                    (+ i 1))))
                    (loop next-i pairs parsed-key "")))))
             ;; Comma separator - end of value
             ((char=? ch #\,)
              (let ((value (if (string? current)
                               (parse-inline-value (string-trim current))
                               current)))  ;; Already parsed (nested object/array)
                ;; Skip space after comma
                (let ((next-i (if (and (< (+ i 1) len)
                                       (char=? (string-ref s (+ i 1)) #\space))
                                  (+ i 2)
                                  (+ i 1))))
                  (loop next-i (cons (cons current-key value) pairs) #f ""))))
             ;; Start of nested object
             ((char=? ch #\{)
              (receive (nested end-i) (parse-nested-inline-object s i)
                (loop end-i pairs current-key nested)))
             ;; Start of nested array
             ((char=? ch #\[)
              (receive (nested end-i) (parse-nested-inline-array s i)
                (loop end-i pairs current-key nested)))
             ;; Start of string
             ((or (char=? ch #\") (char=? ch #\'))
              (receive (str end-i) (parse-inline-string s i ch)
                (loop end-i pairs current-key (string-append current str))))
             ;; Regular character
             (else
              (loop (+ i 1) pairs current-key (string-append current (string ch))))))))))

;; Parse nested inline object
(define (parse-nested-inline-object s start)
  (let loop ((i (+ start 1)) (depth 1) (content "{"))
    (if (or (>= i (string-length s)) (= depth 0))
        (values (parse-inline-object content) i)
        (let ((ch (string-ref s i)))
          (cond
           ((char=? ch #\{)
            (loop (+ i 1) (+ depth 1) (string-append content (string ch))))
           ((char=? ch #\})
            (if (= depth 1)
                (values (parse-inline-object (string-append content "}")) (+ i 1))
                (loop (+ i 1) (- depth 1) (string-append content (string ch)))))
           ((or (char=? ch #\") (char=? ch #\'))
            (receive (str end-i) (parse-inline-string s i ch)
              (loop end-i depth (string-append content str))))
           (else
            (loop (+ i 1) depth (string-append content (string ch)))))))))

;; Parse inline value (for array elements)
(define (parse-inline-value s)
  (let ((s (string-trim s)))
    (cond
     ((string=? s "") 'null)
     ((string=? s "null") 'null)
     ((string=? s "true") #t)
     ((string=? s "false") #f)
     ((string=? s "nan") +nan.0)
     ((string=? s "infinity") +inf.0)
     ((string=? s "-infinity") -inf.0)
     ((string-starts-with? s "\"") (parse-json-string s))
     ((string-starts-with? s "'") (parse-single-quoted-string s))
     ((string-starts-with? s "<")
      (if (string-contains? s ">")
          (parse-angle-bytes s)
          (yay-error "Unmatched angle bracket")))
     ((string-starts-with? s "[") (parse-inline-array s))
     ((string-starts-with? s "{") (parse-inline-object s))
     (else
      (let ((num (parse-number s)))
        (if num
            num
            ;; Not a keyword, string, or number - reject bare words
            (if (> (string-length s) 0)
                (yay-error (string-append "Unexpected character \"" (string (string-ref s 0)) "\""))
                'null)))))))

;; Validate unquoted key characters (alphanumeric, underscore, and hyphen)
(define (validate-unquoted-key key-raw)
  (let ((len (string-length key-raw)))
    (let loop ((i 0))
      (when (< i len)
        (let ((ch (string-ref key-raw i)))
          (when (not (or (alpha? ch) (digit? ch) (char=? ch #\_) (char=? ch #\-)))
            (yay-error (string-append "Invalid character in key: \"" (string ch) "\"")))
          (loop (+ i 1)))))))

;; Validate unquoted key for inline objects (produces "Invalid key" error)
(define (validate-unquoted-key-inline key-raw)
  (let ((len (string-length key-raw)))
    (when (= len 0)
      (yay-error "Invalid key"))
    (let loop ((i 0))
      (when (< i len)
        (let ((ch (string-ref key-raw i)))
          (when (not (or (alpha? ch) (digit? ch) (char=? ch #\_) (char=? ch #\-)))
            (yay-error "Invalid key"))
          (loop (+ i 1)))))))

;; Validate multiline object property syntax
(define (validate-object-property key-raw value-slice)
  ;; Check for space before colon
  (when (and (> (string-length key-raw) 0)
             (char=? (string-ref key-raw (- (string-length key-raw) 1)) #\space))
    (yay-error "Unexpected space before \":\""))
  ;; Validate unquoted key characters
  (when (and (> (string-length key-raw) 0)
             (not (char=? (string-ref key-raw 0) #\"))
             (not (char=? (string-ref key-raw 0) #\')))
    (validate-unquoted-key key-raw))
  ;; Check for exactly one space after colon (or empty for nested)
  (when (> (string-length value-slice) 0)
    (cond
     ;; No space after colon
     ((not (char=? (string-ref value-slice 0) #\space))
      (yay-error "Expected space after \":\""))
     ;; Double space after colon
     ((and (> (string-length value-slice) 1)
           (char=? (string-ref value-slice 1) #\space))
      (yay-error "Unexpected space after \":\"")))))

;; Split key:value
(define (split-key-value s)
  (let ((colon-idx (string-index s #\:)))
    (if (not colon-idx)
        #f
        (let* ((key-raw (substring s 0 colon-idx))
               (value-slice (substring s (+ colon-idx 1) (string-length s))))
          ;; Validate property syntax
          (validate-object-property key-raw value-slice)
          ;; Parse key
          (let ((key (cond
                      ((string-starts-with? key-raw "\"")
                       (parse-json-string key-raw))
                      ((string-starts-with? key-raw "'")
                       (parse-single-quoted-string key-raw))
                      (else key-raw))))
            ;; Parse value part (skip one leading space if present)
            (let ((value-part (if (and (> (string-length value-slice) 0)
                                       (char=? (string-ref value-slice 0) #\space))
                                  (substring value-slice 1 (string-length value-slice))
                                  value-slice)))
              (list key value-part)))))))

;; Parse scalar value
(define (parse-scalar s)
  (cond
   ((string=? s "null") 'null)
   ((string=? s "true") #t)
   ((string=? s "false") #f)
   ((string=? s "nan") +nan.0)
   ((string=? s "infinity") +inf.0)
   ((string=? s "-infinity") -inf.0)
   ((string-starts-with? s "\"") (parse-json-string s))
   ((string-starts-with? s "'") (parse-single-quoted-string s))
   ((string-starts-with? s "<")
    (if (string-contains? s ">")
        (parse-angle-bytes s)
        (yay-error "Unterminated byte literal")))
   ((string-starts-with? s "[") (parse-inline-array s))
   ((string-starts-with? s "{") (parse-inline-object s))
   (else
    (let ((num (parse-number s)))
      (if num
          num
          ;; Not a keyword, string, or number - reject bare words
          (if (> (string-length s) 0)
              (yay-error (string-append "Unexpected character \"" (string (string-ref s 0)) "\""))
              'null))))))

;; Main parser state
(define (make-parser tokens)
  (vector tokens 0))

(define (parser-tokens p) (vector-ref p 0))
(define (parser-idx p) (vector-ref p 1))
(define (parser-idx-set! p i) (vector-set! p 1 i))

(define (parser-peek p)
  (let ((tokens (parser-tokens p))
        (idx (parser-idx p)))
    (if (>= idx (length tokens))
        #f
        (list-ref tokens idx))))

(define (parser-advance! p)
  (parser-idx-set! p (+ (parser-idx p) 1)))

(define (parser-skip-breaks! p)
  (let loop ()
    (let ((t (parser-peek p)))
      (when (and t (or (eq? (token-type t) 'break)
                       (eq? (token-type t) 'stop)))
        (parser-advance! p)
        (loop)))))

;; Check if text starts with "- " (inline bullet)
(define (inline-bullet? s)
  (string-starts-with? s "- "))

;; Recursively parse nested inline bullets like "- - - value"
(define (parse-nested-inline-bullet text)
  (if (inline-bullet? text)
      (let ((inner-text (substring text 2 (string-length text))))
        (let ((inner-val (parse-nested-inline-bullet inner-text)))
          (vector inner-val)))
      (parse-scalar text)))

;; Parse inline bullet list from text like "- a"
(define (parse-inline-bullet-list p base-indent)
  (let loop ((items '()))
    (let ((t (parser-peek p)))
      (cond
       ;; Text starting with "- "
       ((and t (eq? (token-type t) 'text)
             (>= (token-indent t) base-indent)
             (inline-bullet? (token-text t)))
        (let ((value-str (substring (token-text t) 2 (string-length (token-text t)))))
          (parser-advance! p)
          ;; Use parse-nested-inline-bullet to handle "- - - value"
          (loop (cons (parse-nested-inline-bullet value-str) items))))
       ;; More items at deeper indent via start token
       ((and t (eq? (token-type t) 'start)
             (string=? (token-text t) "-")
             (> (token-indent t) base-indent))
        (parser-advance! p)
        (parser-skip-breaks! p)
        (let ((next (parser-peek p)))
          (if (and next (eq? (token-type next) 'text))
              (let ((value (parse-value p)))
                (let skip-stops ()
                  (let ((st (parser-peek p)))
                    (when (and st (eq? (token-type st) 'stop))
                      (parser-advance! p)
                      (skip-stops))))
                (loop (cons value items)))
              (loop items))))
       (else
        (list->vector (reverse items)))))))

;; Parse list array (- items)
(define (parse-list-array p)
  (parse-list-array-impl p -1))

;; Parse list array with min-indent constraint
;; If min-indent >= 0, stop when we see a list item at lower indent
(define (parse-list-array-impl p min-indent)
  (let loop ((items '()))
    (let ((t (parser-peek p)))
      (if (and t (eq? (token-type t) 'start) (string=? (token-text t) "-"))
          (let ((list-indent (token-indent t)))
            ;; Stop if we encounter a list item at a lower indent than expected
            (if (and (>= min-indent 0) (< list-indent min-indent))
                (list->vector (reverse items))
                (begin
                  (parser-advance! p)
                  (parser-skip-breaks! p)
                  (let ((next (parser-peek p)))
                    (cond
                     ;; Nested list via start token
                     ((and next (eq? (token-type next) 'start) (string=? (token-text next) "-"))
                      (let ((nested (parse-list-array-impl p -1)))
                        ;; Skip stops and breaks
                        (let skip-stops-and-breaks ()
                          (let ((t (parser-peek p)))
                            (when (and t (or (eq? (token-type t) 'stop)
                                             (eq? (token-type t) 'break)))
                              (parser-advance! p)
                              (skip-stops-and-breaks))))
                        (loop (cons nested items))))
                     ;; Inline bullet list "- a" as text
                     ((and next (eq? (token-type next) 'text)
                           (inline-bullet? (token-text next)))
                      (let ((nested (parse-inline-bullet-list p list-indent)))
                        ;; Skip stops and breaks
                        (let skip-stops-and-breaks ()
                          (let ((t (parser-peek p)))
                            (when (and t (or (eq? (token-type t) 'stop)
                                             (eq? (token-type t) 'break)))
                              (parser-advance! p)
                              (skip-stops-and-breaks))))
                        (loop (cons nested items))))
                     ;; Text value - check if it's a key:value that might have siblings
                     ((and next (eq? (token-type next) 'text))
                      (let* ((text (token-text next))
                             (text-indent (token-indent next)))
                        (if (and (string-index text #\:)
                                 (not (string-starts-with? text "\""))
                                 (not (string-starts-with? text "'"))
                                 (not (string-starts-with? text "{")))
                            ;; It's a key:value - parse as object block
                            ;; Use list-indent + 2 as base to capture sibling properties
                            (let ((obj (parse-object-block-in-array p text-indent list-indent)))
                              ;; Skip stops and breaks
                              (let skip-stops-and-breaks ()
                                (let ((t (parser-peek p)))
                                  (when (and t (or (eq? (token-type t) 'stop)
                                                   (eq? (token-type t) 'break)))
                                    (parser-advance! p)
                                    (skip-stops-and-breaks))))
                              (loop (cons obj items)))
                            ;; Regular value - pass list-indent for block string termination
                            (let ((value (parse-value p list-indent)))
                              ;; Skip stops and breaks
                              (let skip-stops-and-breaks ()
                                (let ((t (parser-peek p)))
                                  (when (and t (or (eq? (token-type t) 'stop)
                                                   (eq? (token-type t) 'break)))
                                    (parser-advance! p)
                                    (skip-stops-and-breaks))))
                              (loop (cons value items))))))
                     (else
                      ;; Skip stops and breaks and continue
                      (let skip-stops-and-breaks ()
                        (let ((t (parser-peek p)))
                          (when (and t (or (eq? (token-type t) 'stop)
                                           (eq? (token-type t) 'break)))
                            (parser-advance! p)
                            (skip-stops-and-breaks))))
                      (loop items)))))))
          (list->vector (reverse items))))))

;; Parse block string
(define (parse-block-string p first-line)
  (parse-block-string-with-indent p first-line -1))

;; Parse block string with base indent constraint
;; If base-indent >= 0, stop when we see a line at or below that indent
(define (parse-block-string-with-indent p first-line base-indent)
  (parser-advance! p)
  (let loop ((lines (if (string=? first-line "") '() (list first-line))))
    (let ((t (parser-peek p)))
      (cond
       ((not t)
        (finish-block-string lines first-line))
       ((eq? (token-type t) 'break)
        (parser-advance! p)
        (loop (cons "" lines)))
       ((and (eq? (token-type t) 'text)
             (>= base-indent 0)
             (<= (token-indent t) base-indent))
        ;; Stop: line at or below base indent
        (finish-block-string lines first-line))
       ((eq? (token-type t) 'text)
        (parser-advance! p)
        (loop (cons (token-text t) lines)))
       (else
        (finish-block-string lines first-line))))))

(define (finish-block-string lines first-line)
  (let* ((rev-lines (reverse lines))
         ;; Trim trailing empty lines
         (trimmed (let trim-end ((lst rev-lines))
                    (if (and (pair? lst)
                             (pair? (reverse lst))
                             (string=? (car (reverse lst)) ""))
                        (trim-end (reverse (cdr (reverse lst))))
                        lst)))
         ;; Trim leading empty lines
         (trimmed2 (let trim-start ((lst trimmed))
                     (if (and (pair? lst) (string=? (car lst) ""))
                         (trim-start (cdr lst))
                         lst)))
         (body (string-join trimmed2 "\n"))
         (leading-nl (and (string=? first-line "") (pair? trimmed2)))
         (result (string-append (if leading-nl "\n" "")
                                body
                                (if (pair? trimmed2) "\n" ""))))
    (if (string=? result "")
        (error "Empty block string not allowed (use \"\" or \"\\n\" explicitly)")
        result)))

;; Parse block bytes (> hex)
(define (parse-block-bytes p first-line)
  (let ((hex-start (if (string-starts-with? first-line "> ")
                       (substring first-line 2 (string-length first-line))
                       (substring first-line 1 (string-length first-line)))))
    ;; Remove comment if present
    (let ((hex-clean (let ((hash-idx (string-index hex-start #\#)))
                       (if hash-idx
                           (substring hex-start 0 hash-idx)
                           hex-start))))
      ;; Validate: empty leader (just >) without comment is error
      (when (and (string=? (string-trim hex-clean) "")
                 (not (string-index hex-start #\#)))
        (yay-error "Expected hex or comment in hex block"))
      (parser-advance! p)
      (let loop ((hex hex-clean) (base-indent 0))
        (let ((t (parser-peek p)))
          (if (and t
                   (eq? (token-type t) 'text)
                   (> (token-indent t) base-indent))
              (let* ((line (token-text t))
                     (line-clean (let ((hash-idx (string-index line #\#)))
                                   (if hash-idx
                                       (substring line 0 hash-idx)
                                       line))))
                (parser-advance! p)
                (loop (string-append hex line-clean) base-indent))
              (parse-hex-bytes hex)))))))

;; Parse concatenated quoted strings (multiple quoted strings on consecutive lines)
;; Returns #f if there's only one string (single string on new line is invalid)
(define (parse-concatenated-strings p base-indent)
  (let loop ((parts '()))
    (let ((t (parser-peek p)))
      (cond
       ((not t)
        (if (>= (length parts) 2)
            (apply string-append (reverse parts))
            #f))
       ((or (eq? (token-type t) 'break)
            (eq? (token-type t) 'stop))
        (parser-advance! p)
        (loop parts))
       ((and (eq? (token-type t) 'text)
             (< (token-indent t) base-indent))
        (if (>= (length parts) 2)
            (apply string-append (reverse parts))
            #f))
       ((and (eq? (token-type t) 'text)
             (>= (token-indent t) base-indent))
        (let ((trimmed (string-trim (token-text t))))
          (cond
           ;; Double-quoted string
           ((and (>= (string-length trimmed) 2)
                 (char=? (string-ref trimmed 0) #\")
                 (char=? (string-ref trimmed (- (string-length trimmed) 1)) #\"))
            (let ((parsed (parse-json-string trimmed)))
              (parser-advance! p)
              (loop (cons parsed parts))))
           ;; Single-quoted string
           ((and (>= (string-length trimmed) 2)
                 (char=? (string-ref trimmed 0) #\')
                 (char=? (string-ref trimmed (- (string-length trimmed) 1)) #\'))
            (let ((parsed (parse-single-quoted-string trimmed)))
              (parser-advance! p)
              (loop (cons parsed parts))))
           ;; Not a quoted string - stop
           (else
            (if (>= (length parts) 2)
                (apply string-append (reverse parts))
                #f)))))
       (else
        (if (>= (length parts) 2)
            (apply string-append (reverse parts))
            #f))))))

;; Parse block bytes in property context
(define (parse-property-block-bytes p first-line base-indent)
  (let ((hex-start (if (string-starts-with? first-line "> ")
                       (substring first-line 2 (string-length first-line))
                       (substring first-line 1 (string-length first-line)))))
    ;; Remove comment if present
    (let ((hex-clean (let ((hash-idx (string-index hex-start #\#)))
                       (if hash-idx
                           (substring hex-start 0 hash-idx)
                           hex-start))))
      ;; Validate: no hex content on same line as property (must be on next line)
      (when (> (string-length (string-trim hex-clean)) 0)
        (yay-error "Expected newline after block leader in property"))
      (parser-advance! p)
      (let loop ((hex hex-clean))
        (let ((t (parser-peek p)))
          (if (and t
                   (eq? (token-type t) 'text)
                   (> (token-indent t) base-indent))
              (let* ((line (token-text t))
                     (line-clean (let ((hash-idx (string-index line #\#)))
                                   (if hash-idx
                                       (substring line 0 hash-idx)
                                       line))))
                (parser-advance! p)
                (loop (string-append hex line-clean)))
              (parse-hex-bytes hex)))))))

;; Parse multiline angle bytes
(define (parse-multiline-angle-bytes p first-line base-indent)
  (let ((hex-start (if (string-starts-with? first-line "< ")
                       (substring first-line 2 (string-length first-line))
                       (substring first-line 1 (string-length first-line)))))
    ;; Remove comment if present
    (let ((hex-clean (let ((hash-idx (string-index hex-start #\#)))
                       (if hash-idx
                           (substring hex-start 0 hash-idx)
                           hex-start))))
      (parser-advance! p)
      (let loop ((hex hex-clean))
        (let ((t (parser-peek p)))
          (if (and t
                   (eq? (token-type t) 'text)
                   (> (token-indent t) base-indent))
              (let* ((line (token-text t))
                     (line-clean (let ((hash-idx (string-index line #\#)))
                                   (if hash-idx
                                       (substring line 0 hash-idx)
                                       line))))
                (parser-advance! p)
                (loop (string-append hex line-clean)))
              (parse-hex-bytes hex)))))))

;; Parse value with optional base indent for block strings in array context
(define (parse-value p . args)
  (let ((base-indent (if (pair? args) (car args) -1)))
    (parse-value-with-indent p base-indent)))

(define (parse-value-with-indent p base-indent)
  (let ((t (parser-peek p)))
    (if (not t)
        'null
        (case (token-type t)
          ((start)
           (cond
            ((string=? (token-text t) "-")
             (parse-list-array p))
            (else
             (parser-advance! p)
             (parse-value-with-indent p base-indent))))
          ((text)
           (let ((s (token-text t)))
             (cond
              ;; Leading space error
              ((string-starts-with? s " ")
               (yay-error "Unexpected leading space"))
              ;; Dollar sign error
              ((string=? s "$")
               (yay-error "Unexpected character \"$\""))
              ;; Keywords
              ((string=? s "null")
               (parser-advance! p)
               'null)
              ((string=? s "true")
               (parser-advance! p)
               #t)
              ((string=? s "false")
               (parser-advance! p)
               #f)
              ((string=? s "nan")
               (parser-advance! p)
               +nan.0)
              ((string=? s "infinity")
               (parser-advance! p)
               +inf.0)
              ((string=? s "-infinity")
               (parser-advance! p)
               -inf.0)
              ;; Block string (backtick)
              ((or (string=? s "`")
                   (and (string-starts-with? s "`")
                        (>= (string-length s) 2)
                        (char=? (string-ref s 1) #\space)))
               (let ((first-line (if (> (string-length s) 2)
                                     (substring s 2 (string-length s))
                                     "")))
                 (parse-block-string-with-indent p first-line base-indent)))
              ;; Block bytes (>)
              ((and (string-starts-with? s ">")
                    (not (string-contains? s "<")))
               (parse-block-bytes p s))
              ;; Quoted string
              ((and (string-starts-with? s "\"") (string-ends-with? s "\""))
               (parser-advance! p)
               (parse-json-string s))
              ((and (string-starts-with? s "'") (string-ends-with? s "'"))
               (parser-advance! p)
               (parse-single-quoted-string s))
              ;; Inline array
              ((string-starts-with? s "[")
               (parser-advance! p)
               (parse-inline-array s))
              ;; Angle bytes (must be complete with closing >)
              ((string-starts-with? s "<")
               (if (string-contains? s ">")
                   (begin
                     (parser-advance! p)
                     (parse-angle-bytes s))
                   (yay-error "Unmatched angle bracket")))
              ;; Inline object
              ((string-starts-with? s "{")
               (parser-advance! p)
               (parse-inline-object s))
              ;; Key:value
              ((string-index s #\:)
               (let ((kv (split-key-value s)))
                 (if kv
                     (let ((key (car kv))
                           (value-part (cadr kv)))
                       (cond
                        ;; Empty value - nested object or array
                        ((string=? value-part "")
                         (parser-advance! p)
                         (parser-skip-breaks! p)
                         (let ((next (parser-peek p)))
                           (cond
                            ;; Array - pass next.indent so array stops at items below this level
                            ((and next (eq? (token-type next) 'start)
                                  (string=? (token-text next) "-"))
                             (let ((arr (parse-list-array-impl p (token-indent next))))
                               (list (cons key arr))))
                            ;; Nested object
                            ((and next (eq? (token-type next) 'text))
                             (let ((obj (parse-object-block p (token-indent next))))
                               (list (cons key obj))))
                            ;; Block string (backtick)
                            ((and next (eq? (token-type next) 'text)
                                  (string=? (token-text next) "`"))
                             (let ((str (parse-block-string p "")))
                               (list (cons key str))))
                            ;; Block bytes on next line
                            ((and next (eq? (token-type next) 'text)
                                  (string-starts-with? (token-text next) ">")
                                  (not (string-contains? (token-text next) "<")))
                             (let ((bytes (parse-block-bytes p (token-text next))))
                               (list (cons key bytes))))
                            ;; No next token - error (empty value at end of document)
                            ((not next)
                             (yay-error "Expected value after property"))
                            (else
                             (list (cons key 'null))))))
                        ;; Empty object
                        ((string=? value-part "{}")
                         (parser-advance! p)
                         (list (cons key '())))
                        ;; Angle bytes (< must have closing >)
                        ((string-starts-with? value-part "<")
                         (if (string-contains? value-part ">")
                             (begin
                               (parser-advance! p)
                               (list (cons key (parse-angle-bytes value-part))))
                             (yay-error "Unmatched angle bracket")))
                        ;; Block string as property (backtick)
                        ((or (string=? value-part "`")
                             (and (string-starts-with? value-part "`")
                                  (>= (string-length value-part) 2)
                                  (char=? (string-ref value-part 1) #\space)))
                         (parser-advance! p)
                         (let ((first-line (if (> (string-length value-part) 2)
                                               (substring value-part 2 (string-length value-part))
                                               "")))
                           (let ((str (parse-block-string-for-property p first-line (token-indent t))))
                             (list (cons key str)))))
                        ;; Block bytes as property
                        ((and (string-starts-with? value-part ">")
                              (not (string-contains? value-part "<")))
                         (let ((bytes (parse-property-block-bytes p value-part (token-indent t))))
                           (list (cons key bytes))))
                        ;; Regular value
                        (else
                         (parser-advance! p)
                         (list (cons key (parse-scalar value-part))))))
                     (begin
                       (parser-advance! p)
                       (parse-scalar s)))))
              ;; Number
              (else
               (let ((num (parse-number s)))
                 (parser-advance! p)
                 (if num num (parse-scalar s)))))))
          ((stop break)
           (parser-advance! p)
           (parse-value p))
          (else
           (parser-advance! p)
           'null)))))

;; Parse block string for property (handles indentation)
(define (parse-block-string-for-property p first-line base-indent)
  ;; Validate: no content on same line as backtick
  (when (> (string-length first-line) 0)
    (yay-error "Expected newline after block leader in property"))
  (let loop ((lines (if (string=? first-line "") '() (list (cons 0 first-line))))
             (min-indent #f))
    (let ((t (parser-peek p)))
      (cond
       ((not t)
        (finish-block-string-property lines))
       ((eq? (token-type t) 'break)
        (parser-advance! p)
        (loop (cons (cons #f "") lines) min-indent))
       ((and (eq? (token-type t) 'text)
             (> (token-indent t) base-indent))
        (let ((indent (token-indent t)))
          (parser-advance! p)
          (loop (cons (cons indent (token-text t)) lines)
                (if (or (not min-indent) (< indent min-indent))
                    indent
                    min-indent))))
       (else
        (finish-block-string-property lines))))))

(define (finish-block-string-property lines)
  (let* ((rev-lines (reverse lines))
         ;; Find minimum indent
         (min-indent (let loop ((lst rev-lines) (min-i #f))
                       (if (null? lst)
                           (or min-i 0)
                           (let ((indent (car (car lst))))
                             (if (and indent (or (not min-i) (< indent min-i)))
                                 (loop (cdr lst) indent)
                                 (loop (cdr lst) min-i))))))
         ;; Build lines with relative indentation
         (body-lines (map (lambda (entry)
                            (let ((indent (car entry))
                                  (text (cdr entry)))
                              (if indent
                                  (let ((extra (- indent min-indent)))
                                    (string-append (make-string extra #\space) text))
                                  "")))
                          rev-lines))
         ;; Trim trailing empty lines
         (trimmed (let trim-end ((lst body-lines))
                    (if (and (pair? lst)
                             (pair? (reverse lst))
                             (string=? (car (reverse lst)) ""))
                        (trim-end (reverse (cdr (reverse lst))))
                        lst)))
         (result (string-append (string-join trimmed "\n")
                                (if (pair? trimmed) "\n" ""))))
    (if (string=? result "")
        (error "Empty block string not allowed (use \"\" or \"\\n\" explicitly)")
        result)))

;; Parse object block inside an array item
;; first-indent is the indent of the first key (right after -)
;; array-indent is the indent of the array item marker
;; We accept keys at first-indent OR at array-indent + 2
(define (parse-object-block-in-array p first-indent array-indent)
  (let ((sibling-indent (+ array-indent 2)))
    (let loop ((pairs '()))
      (let ((t (parser-peek p)))
        (cond
         ((not t)
          (reverse pairs))
         ;; Stop if we see a start token (array item) at or below array indent
         ((and (eq? (token-type t) 'start)
               (<= (token-indent t) array-indent))
          (reverse pairs))
         ((eq? (token-type t) 'stop)
          (parser-advance! p)
          (loop pairs))
         ((eq? (token-type t) 'break)
          (parser-advance! p)
          (loop pairs))
         ;; Accept text at first-indent or sibling-indent
         ((and (eq? (token-type t) 'text)
               (or (= (token-indent t) first-indent)
                   (= (token-indent t) sibling-indent)))
          (let* ((s (token-text t))
                 (current-indent (token-indent t))
                 (kv (split-key-value s)))
            (if kv
                (let ((key (car kv))
                      (value-part (cadr kv)))
                  (cond
                   ;; Empty value - nested
                   ((string=? value-part "")
                    (parser-advance! p)
                    (parser-skip-breaks! p)
                    (let ((next (parser-peek p)))
                      (cond
                       ((and next (eq? (token-type next) 'start)
                             (string=? (token-text next) "-"))
                        (let ((arr (parse-list-array-impl p (token-indent next))))
                          (loop (cons (cons key arr) pairs))))
                       ((and next (eq? (token-type next) 'text)
                             (> (token-indent next) current-indent))
                        (let ((obj (parse-object-block p (token-indent next))))
                          (loop (cons (cons key obj) pairs))))
                       ;; Empty value is always an error
                       (else
                        (yay-error "Expected value after property")))))
                   ;; Empty object
                   ((string=? value-part "{}")
                    (parser-advance! p)
                    (loop (cons (cons key '()) pairs)))
                   ;; Angle bytes (< must have closing >)
                   ((string-starts-with? value-part "<")
                    (if (string-contains? value-part ">")
                        (begin
                          (parser-advance! p)
                          (loop (cons (cons key (parse-angle-bytes value-part)) pairs)))
                        (yay-error "Unmatched angle bracket")))
                   ;; Regular value
                   (else
                    (parser-advance! p)
                    (loop (cons (cons key (parse-scalar value-part)) pairs)))))
                (begin
                  (parser-advance! p)
                  (loop pairs)))))
         ;; Text at different indent - stop
         ((eq? (token-type t) 'text)
          (reverse pairs))
         (else
          (parser-advance! p)
          (loop pairs)))))))

;; Parse object block
(define (parse-object-block p base-indent)
  (let loop ((pairs '()))
    (let ((t (parser-peek p)))
      (cond
       ((not t)
        (reverse pairs))
       ;; Stop if we see a start token (array item) at or below our indent
       ((and (eq? (token-type t) 'start)
             (<= (token-indent t) base-indent))
        (reverse pairs))
       ((eq? (token-type t) 'stop)
        (parser-advance! p)
        (loop pairs))
       ((eq? (token-type t) 'break)
        (parser-advance! p)
        (loop pairs))
       ((and (eq? (token-type t) 'text)
             (< (token-indent t) base-indent))
        (reverse pairs))
       ((and (eq? (token-type t) 'text)
             (= (token-indent t) base-indent))
        (let* ((s (token-text t))
               (kv (split-key-value s)))
          (if kv
              (let ((key (car kv))
                    (value-part (cadr kv)))
                (cond
                 ;; Empty value - nested
                 ((string=? value-part "")
                  (parser-advance! p)
                  (parser-skip-breaks! p)
                  (let ((next (parser-peek p)))
                    (cond
                     ((and next (eq? (token-type next) 'start)
                           (string=? (token-text next) "-"))
                      (let ((arr (parse-list-array-impl p (token-indent next))))
                        (loop (cons (cons key arr) pairs))))
                     ((and next (eq? (token-type next) 'text)
                           (> (token-indent next) base-indent))
                      ;; Check for concatenated quoted strings first
                      (let ((trimmed (string-trim (token-text next))))
                        (if (and (>= (string-length trimmed) 2)
                                 (or (and (char=? (string-ref trimmed 0) #\")
                                          (char=? (string-ref trimmed (- (string-length trimmed) 1)) #\"))
                                     (and (char=? (string-ref trimmed 0) #\')
                                          (char=? (string-ref trimmed (- (string-length trimmed) 1)) #\'))))
                            ;; Try to parse concatenated strings
                            (let ((result (parse-concatenated-strings p (token-indent next))))
                              (if result
                                  (loop (cons (cons key result) pairs))
                                  ;; Single string on new line is invalid
                                  (yay-error "Unexpected indent")))
                            ;; Not a quoted string - parse as nested object
                            (let ((obj (parse-object-block p (token-indent next))))
                              (loop (cons (cons key obj) pairs))))))
                     ((and next (eq? (token-type next) 'text)
                           (or (string=? (token-text next) "\"")
                               (and (string-starts-with? (token-text next) "\"")
                                    (>= (string-length (token-text next)) 2)
                                    (char=? (string-ref (token-text next) 1) #\space))))
                      (let ((first-line (if (> (string-length (token-text next)) 2)
                                            (substring (token-text next) 2 (string-length (token-text next)))
                                            "")))
                        (parser-advance! p)
                        (let ((str (parse-block-string-for-property p first-line base-indent)))
                          (loop (cons (cons key str) pairs)))))
                     ((and next (eq? (token-type next) 'text)
                           (string-starts-with? (token-text next) "<"))
                      ;; Angle bytes on next line - must have closing >
                      (if (string-contains? (token-text next) ">")
                          (begin
                            (parser-advance! p)
                            (loop (cons (cons key (parse-angle-bytes (token-text next))) pairs)))
                          (yay-error "Unmatched angle bracket")))
                     ;; Empty value is always an error
                     (else
                      (yay-error "Expected value after property")))))
                 ;; Empty object
                 ((string=? value-part "{}")
                  (parser-advance! p)
                  (loop (cons (cons key '()) pairs)))
                 ;; Angle bytes (< must have closing >)
                 ((string-starts-with? value-part "<")
                  (if (string-contains? value-part ">")
                      (begin
                        (parser-advance! p)
                        (loop (cons (cons key (parse-angle-bytes value-part)) pairs)))
                      (yay-error "Unmatched angle bracket")))
                 ;; Block string (backtick)
                 ((or (string=? value-part "`")
                      (and (string-starts-with? value-part "`")
                           (>= (string-length value-part) 2)
                           (char=? (string-ref value-part 1) #\space)))
                  (parser-advance! p)
                  (let ((first-line (if (> (string-length value-part) 2)
                                        (substring value-part 2 (string-length value-part))
                                        "")))
                    (let ((str (parse-block-string-for-property p first-line base-indent)))
                      (loop (cons (cons key str) pairs)))))
                 ;; Block bytes as property
                 ((and (string-starts-with? value-part ">")
                       (not (string-contains? value-part "<")))
                  (let ((bytes (parse-property-block-bytes p value-part base-indent)))
                    (loop (cons (cons key bytes) pairs))))
                 ;; Regular value
                 (else
                  (parser-advance! p)
                  (loop (cons (cons key (parse-scalar value-part)) pairs)))))
              (begin
                (parser-advance! p)
                (loop pairs)))))
       ((and (eq? (token-type t) 'text)
             (> (token-indent t) base-indent))
        (parser-advance! p)
        (loop pairs))
       (else
        (parser-advance! p)
        (loop pairs))))))

;; Parse root
(define (parse-root p)
  (parser-skip-breaks! p)
  (let ((t (parser-peek p)))
    (if (not t)
        (yay-error "No value found in document")
        (let* ((is-root-object (and (eq? (token-type t) 'text)
                                    (string-index (token-text t) #\:)
                                    (= (token-indent t) 0)
                                    ;; Don't treat inline objects as root objects
                                    (not (string-starts-with? (token-text t) "{"))))
               (result (if is-root-object
                           ;; Root object
                           (parse-object-block p 0)
                           ;; Single value
                           (parse-value p))))
          ;; Check for extra content after single values (not root objects)
          (when (not is-root-object)
            (parser-skip-breaks! p)
            (let ((remaining (parser-peek p)))
              (when remaining
                (yay-error "Unexpected extra content"))))
          result))))

;; Main entry point
(define (parse-yay source)
  (let* ((lines (scan source))
         (tokens (tokenize lines))
         (parser (make-parser tokens)))
    (parse-root parser)))

;; Check if value is a bytevector (tagged)
(define (bytevector? v)
  (and (pair? v) (eq? (car v) 'bytevector)))

(define (bytevector-bytes v)
  (cdr v))

;; Convert YAY value to Scheme string representation
(define (yay->scheme-string value)
  (cond
   ((eq? value 'null) "'null")
   ((eq? value #t) "#t")
   ((eq? value #f) "#f")
   ((my-nan? value) "+nan.0")
   ((my-infinite? value)
    (if (> value 0) "+inf.0" "-inf.0"))
   ((and (number? value) (exact? value)) (number->string value))
   ((and (number? value) (inexact? value))
    (let ((s (number->string value)))
      ;; Ensure there's a decimal point
      (if (string-index s #\.)
          s
          (string-append s ".0"))))
   ((string? value) (format-scheme-string value))
   ((bytevector? value)
    (format-bytevector (bytevector-bytes value)))
   ((vector? value)
    ;; Regular array
    (string-append "#("
                   (string-join (map yay->scheme-string (vector->list value)) " ")
                   ")"))
   ((list? value)
    ;; Alist (object)
    (if (null? value)
        "()"
        (string-append "("
                       (string-join
                        (map (lambda (pair)
                               (string-append "("
                                              (format-scheme-string (car pair))
                                              " . "
                                              (yay->scheme-string (cdr pair))
                                              ")"))
                             value)
                        " ")
                       ")")))
   (else (let ((port (open-output-string)))
           (write value port)
           (get-output-string port)))))

;; Format string for Scheme output
(define (format-scheme-string s)
  (string-append "\""
                 (let loop ((i 0) (acc ""))
                   (if (>= i (string-length s))
                       acc
                       (let ((c (string-ref s i))
                             (code (char->integer (string-ref s i))))
                         (cond
                          ((char=? c #\") (loop (+ i 1) (string-append acc "\\\"")))
                          ((char=? c #\\) (loop (+ i 1) (string-append acc "\\\\")))
                          ((char=? c #\/) (loop (+ i 1) (string-append acc "\\/")))
                          ((char=? c #\newline) (loop (+ i 1) (string-append acc "\\n")))
                          ((char=? c #\return) (loop (+ i 1) (string-append acc "\\r")))
                          ((char=? c #\tab) (loop (+ i 1) (string-append acc "\\t")))
                          ((char=? c #\backspace) (loop (+ i 1) (string-append acc "\\b")))
                          ((char=? c (integer->char 12)) (loop (+ i 1) (string-append acc "\\f")))
                          ;; Non-ASCII characters - keep as-is if fixture expects it
                          ;; (emojis and other unicode are kept as literal characters)
                          ((> code 127)
                           (let ((hex (number->string code 16)))
                             (if (<= (string-length hex) 4)
                                 ;; 4 or fewer hex digits - use \uXXXX format
                                 (loop (+ i 1) 
                                       (string-append acc "\\u" 
                                                      (string-append 
                                                       (make-string (- 4 (string-length hex)) #\0)
                                                       (string-upcase hex))))
                                 ;; More than 4 hex digits (emoji etc) - keep as literal
                                 (loop (+ i 1) (string-append acc (string c))))))
                          (else (loop (+ i 1) (string-append acc (string c))))))))
                 "\""))

;; Format bytevector
(define (format-bytevector bytes)
  (string-append "(bytevector"
                 (if (null? bytes)
                     ""
                     (string-append " "
                                    (string-join
                                     (map number->string bytes)
                                     " ")))
                 ")"))

;; ============================================================================
;; YAY Equality
;; ============================================================================
;; Structural equality for YAY values.
;; - Alists (objects) are compared as unordered key-value maps
;; - Vectors (arrays) are compared element-by-element in order
;; - NaN is equal to NaN (unlike standard numeric equality)
;; - Bytevectors are compared byte-by-byte

(define (yay-equal? a b)
  (cond
   ;; Both null
   ((and (eq? a 'null) (eq? b 'null)) #t)
   ;; Both booleans
   ((and (boolean? a) (boolean? b)) (eq? a b))
   ;; Both NaN (special case: NaN = NaN for YAY equality)
   ((and (my-nan? a) (my-nan? b)) #t)
   ;; Both numbers (including infinity)
   ((and (number? a) (number? b)) (= a b))
   ;; Both strings
   ((and (string? a) (string? b)) (string=? a b))
   ;; Both bytevectors
   ((and (bytevector? a) (bytevector? b))
    (bytevector-equal? (bytevector-bytes a) (bytevector-bytes b)))
   ;; Both vectors (arrays) - order matters
   ((and (vector? a) (vector? b))
    (vector-equal? a b))
   ;; Both alists (objects) - order does NOT matter
   ((and (alist? a) (alist? b))
    (alist-equal? a b))
   ;; Type mismatch
   (else #f)))

;; Check if value is an alist (list of pairs with string keys)
(define (alist? v)
  (and (list? v)
       (or (null? v)
           (and (pair? (car v))
                (string? (caar v))))))

;; Compare two bytevectors byte-by-byte
(define (bytevector-equal? a b)
  (and (= (length a) (length b))
       (let loop ((as a) (bs b))
         (or (null? as)
             (and (= (car as) (car bs))
                  (loop (cdr as) (cdr bs)))))))

;; Compare two vectors element-by-element (order matters)
(define (vector-equal? a b)
  (let ((len-a (vector-length a))
        (len-b (vector-length b)))
    (and (= len-a len-b)
         (let loop ((i 0))
           (or (>= i len-a)
               (and (yay-equal? (vector-ref a i) (vector-ref b i))
                    (loop (+ i 1))))))))

;; Look up a key in an alist, returns the pair or #f
(define (alist-ref alist key)
  (let loop ((pairs alist))
    (cond
     ((null? pairs) #f)
     ((string=? (caar pairs) key) (car pairs))
     (else (loop (cdr pairs))))))

;; Compare two alists as unordered maps
(define (alist-equal? a b)
  (and (= (length a) (length b))
       ;; Every key in a exists in b with equal value
       (let loop ((pairs a))
         (or (null? pairs)
             (let* ((pair (car pairs))
                    (key (car pair))
                    (val-a (cdr pair))
                    (found (alist-ref b key)))
               (and found
                    (yay-equal? val-a (cdr found))
                    (loop (cdr pairs))))))))
