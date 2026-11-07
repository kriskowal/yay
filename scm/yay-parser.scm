;;; YAY Parser for Scheme
;;; A standalone parser that works with Guile, Chez, Chicken, etc.

(use-modules (ice-9 receive) (srfi srfi-9))

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
;; Tagged values: a tag, a string, and a payload, any value
(define-record-type tagged
  (make-tagged tag payload)
  tagged?
  (tag tagged-tag)
  (payload tagged-payload))

(define prefix-without-block
  "A tag before a value needs a block below it; write the tag after an inline value")

;; References: a URI or IRI, held as written, without its angle brackets.
;; YAY never dereferences one. Two references are equal when their texts are;
;; a reference never equals a string.
(define-record-type reference
  (make-reference uri)
  reference?
  (uri reference-uri))

;; The length of the scheme, colon included, if s at start (the text after
;; "<") begins [A-Za-z][A-Za-z0-9+.-]*":", or 0. Uppercase letters are
;; matched so that the reader can name an uppercase scheme rather than call
;; it bad hex.
(define (scheme-length s start)
  (let ((len (string-length s)))
    (if (or (>= start len) (not (alpha? (string-ref s start))))
        0
        (let loop ((i (+ start 1)))
          (cond
           ((>= i len) 0)
           ((char=? (string-ref s i) #\:) (- (+ i 1) start))
           ((let ((c (string-ref s i)))
              (or (alpha? c) (digit? c) (memv c '(#\+ #\- #\.))))
            (loop (+ i 1)))
           (else 0))))))

;; Whether s at start begins with "<" and a scheme, so is a reference rather
;; than bytes
(define (reference-start? s start)
  (and (< start (string-length s))
       (char=? (string-ref s start) #\<)
       (> (scheme-length s (+ start 1)) 0)))

;; The index past the ">" that closes a reference at s[start], or #f if
;; there is no reference there or it does not close. The scanners for
;; quotes, brackets, colons, tags and comments skip a reference's text with
;; it, since its punctuation is not syntax.
(define (skip-reference s start)
  (and (reference-start? s start)
       (let loop ((i (+ start 1)))
         (cond
          ((>= i (string-length s)) #f)
          ((char=? (string-ref s i) #\>) (+ i 1))
          (else (loop (+ i 1)))))))

;; Whether a character may appear in a reference (RFC 3986 and RFC 3987),
;; "%" aside, which must begin a percent-encoding
(define (uri-char? c)
  (let ((cp (char->integer c)))
    (if (< cp #x80)
        (or (alpha? c) (digit? c)
            (and (memv c (string->list "-._~:/?#[]@!$&'()*+,;=")) #t))
        (and (>= cp #xA0)
             (not (char-whitespace? c))
             (not (memv cp '(#xA0 #x1680 #x2028 #x2029 #x202F #x205F #x3000
                             #xFFFE #xFFFF)))
             (not (and (>= cp #x2000) (<= cp #x200A)))))))

;; Read a reference at s[start], which reference-start? accepted, to the
;; first ">", which always closes it. Returns the reference and the index
;; past the ">".
(define (parse-reference s start)
  (let* ((len (string-length s))
         (n (- (scheme-length s (+ start 1)) 1))
         (scheme (substring s (+ start 1) (+ start 1 n))))
    (unless (string=? scheme (string-downcase scheme))
      (yay-error (string-append "Uppercase scheme \"" scheme
                                "\" in reference (use lowercase)")))
    (let loop ((i (+ start 1 n)))
      (when (>= i len)
        (yay-error "Unmatched angle bracket"))
      (let ((c (string-ref s i)))
        (cond
         ((char=? c #\>)
          (values (make-reference (substring s (+ start 1) i)) (+ i 1)))
         ((char=? c #\%)
          (unless (and (< (+ i 2) len)
                       (hex-digit? (string-ref s (+ i 1)))
                       (hex-digit? (string-ref s (+ i 2))))
            (yay-error "Bad percent-encoding in reference (use % and two hex digits)"))
          (loop (+ i 1)))
         ((memv c '(#\newline #\return))
          (yay-error "Unmatched angle bracket"))
         ((char=? c #\space)
          (yay-error "Unexpected space in reference"))
         ((char=? c #\tab)
          (yay-error "Tab not allowed in reference"))
         ((not (uri-char? c))
          (yay-error (string-append "Unexpected character \"" (string c)
                                    "\" in reference")))
         (else (loop (+ i 1))))))))

;; Read a reference that must be all of s
(define (parse-whole-reference s)
  (receive (ref end) (parse-reference s 0)
    (cond
     ((= end (string-length s)) ref)
     ((char=? (string-ref s end) #\:) (yay-error "Unexpected character \":\""))
     (else
      (yay-error (string-append "Unexpected character \""
                                (string (string-ref s end)) "\""))))))

;; The index past the tag at s[start], "(", or s's length if it does not close
(define (skip-tag s start)
  (let loop ((i (+ start 1)))
    (cond
     ((>= i (string-length s)) (string-length s))
     ((char=? (string-ref s i) #\\) (loop (+ i 2)))
     ((char=? (string-ref s i) #\)) (+ i 1))
     (else (loop (+ i 1))))))

;; Read a tag at s[start], "(", escaped as a double-quoted string is but with
;; \) in the place of \". Returns the tag and the index past the ")".
(define (read-tag s start)
  (let ((len (string-length s)))
    (let loop ((i (+ start 1)) (acc '()))
      (when (>= i len)
        (yay-error "Unterminated string"))
      (let ((c (string-ref s i)))
        (cond
         ((char=? c #\))
          (let* ((end (+ i 1))
                 (after (if (< end len) (string-ref s end) #f)))
            ;; After a tag: the end, an inline container going on, or one
            ;; space before another tag, a block or a comment
            (when (and after (not (memv after '(#\, #\] #\} #\space))))
              (yay-error "Unexpected character after \")\""))
            (when (and after (char=? after #\space)
                       (< (+ end 1) len)
                       (char=? (string-ref s (+ end 1)) #\space)
                       (not (and (< (+ end 2) len)
                                 (memv (string-ref s (+ end 2)) '(#\# #\()))))
              (yay-error "Unexpected space after \")\""))
            (values (list->string (reverse acc)) end)))
         ((char=? c #\tab)
          (yay-error "Tab not allowed (use spaces)"))
         ((not (char=? c #\\))
          (loop (+ i 1) (cons c acc)))
         (else
          (let ((e (if (< (+ i 1) len) (string-ref s (+ i 1)) #f)))
            (case e
              ((#\)) (loop (+ i 2) (cons #\) acc)))
              ((#\\) (loop (+ i 2) (cons #\\ acc)))
              ((#\/) (loop (+ i 2) (cons #\/ acc)))
              ((#\b) (loop (+ i 2) (cons #\backspace acc)))
              ((#\f) (loop (+ i 2) (cons #\page acc)))
              ((#\n) (loop (+ i 2) (cons #\newline acc)))
              ((#\r) (loop (+ i 2) (cons #\return acc)))
              ((#\t) (loop (+ i 2) (cons #\tab acc)))
              ((#\u)
               (unless (and (< (+ i 2) len) (char=? (string-ref s (+ i 2)) #\{))
                 (yay-error "Bad escaped character"))
               (let hex ((j (+ i 3)) (cp 0) (digits 0))
                 (cond
                  ((>= j len) (yay-error "Bad Unicode escape"))
                  ((char=? (string-ref s j) #\})
                   (when (= digits 0) (yay-error "Bad Unicode escape"))
                   (when (and (>= cp #xD800) (<= cp #xDFFF))
                     (yay-error "Illegal surrogate"))
                   (when (> cp #x10FFFF)
                     (yay-error "Unicode code point out of range"))
                   (loop (+ j 1) (cons (integer->char cp) acc)))
                  (else
                   (let ((d (char->digit (string-ref s j))))
                     (when (or (not d) (= digits 6))
                       (yay-error "Bad Unicode escape"))
                     (hex (+ j 1) (+ (* cp 16) d) (+ digits 1)))))))
              (else (yay-error "Bad escaped character"))))))))))

(define (char->digit c)
  (let ((d (string->number (string c) 16)))
    (and d (exact? d) d)))

;; The index of the first tag at s's top level, outside quotes and inline
;; containers, or #f
(define (find-top-level-tag s)
  (let ((len (string-length s)))
    (let loop ((i 0) (quote-char #f) (depth 0))
      (if (>= i len)
          #f
          (let ((c (string-ref s i)))
            (cond
             (quote-char
              (cond
               ((char=? c #\\) (loop (+ i 2) quote-char depth))
               ((char=? c quote-char) (loop (+ i 1) #f depth))
               (else (loop (+ i 1) quote-char depth))))
             ((or (char=? c #\") (char=? c #\')) (loop (+ i 1) c depth))
             ((skip-reference s i) => (lambda (end) (loop end #f depth)))
             ((or (char=? c #\[) (char=? c #\{)) (loop (+ i 1) #f (+ depth 1)))
             ((or (char=? c #\]) (char=? c #\})) (loop (+ i 1) #f (- depth 1)))
             ((char=? c #\()
              (if (= depth 0) i (loop (skip-tag s i) #f depth)))
             (else (loop (+ i 1) #f depth))))))))

;; Parse an inline value with tags after it, innermost first, one space
;; before each; parse-atom reads the value before them
(define (parse-tagged parse-atom s)
  (let ((tag-at (find-top-level-tag s))
        (len (string-length s)))
    (cond
     ((not tag-at) (parse-atom s))
     ((= tag-at 0) (yay-error prefix-without-block))
     ((not (char=? (string-ref s (- tag-at 1)) #\space))
      (yay-error "Expected one space before a tag"))
     ((and (> tag-at 1) (char=? (string-ref s (- tag-at 2)) #\space))
      (yay-error "Unexpected space before \"(\""))
     (else
      (let loop ((value (parse-atom (substring s 0 (- tag-at 1))))
                 (i (- tag-at 1)))
        (cond
         ((= i len) value)
         ((and (< (+ i 1) len)
               (char=? (string-ref s i) #\space)
               (char=? (string-ref s (+ i 1)) #\())
          (receive (tag end) (read-tag s (+ i 1))
            (loop (make-tagged tag value) end)))
         (else
          (yay-error (string-append "Unexpected character \""
                                    (string (string-ref s i)) "\"")))))))))

;; Read the tags before a block, outermost first, at the start of s. Returns
;; the tags and what follows them; a comment after them is nothing.
(define (parse-prefix s)
  (let loop ((i 0) (tags '()))
    (if (and (< i (string-length s)) (char=? (string-ref s i) #\())
        (receive (tag end) (read-tag s i)
          (loop (if (and (< end (string-length s))
                         (char=? (string-ref s end) #\space))
                    (+ end 1)
                    end)
                (cons tag tags)))
        (let ((rest (substring s i (string-length s))))
          (values (reverse tags)
                  (if (string-starts-with? (string-trim rest) "#") "" rest))))))

;; Wrap a block in a prefix's tags, outermost first
(define (wrap-prefix value tags)
  (if (null? tags)
      value
      (make-tagged (car tags) (wrap-prefix value (cdr tags)))))

;; Past the tags, each "(...)" and one space, at the start of content
(define (skip-prefix-tags content)
  (let loop ((i 0))
    (if (and (< i (string-length content)) (char=? (string-ref content i) #\())
        (let ((end (skip-tag content i)))
          (loop (if (and (< end (string-length content))
                         (char=? (string-ref content end) #\space))
                    (+ end 1)
                    end)))
        (substring content i (string-length content)))))

;; Whether a line's content is a property whose tags precede a block string
;; leader: "key: (tag) `"
(define (tagged-property-opener? content)
  (let ((at (string-contains-at content ": (")))
    (and at
         (string-ends-with? content "`")
         (not (string-index (substring content 0 at) #\"))
         (not (string-index (substring content 0 at) #\'))
         (string=? (skip-prefix-tags (substring content (+ at 2) (string-length content)))
                   "`"))))

(define (string-contains-at s sub)
  (let ((n (string-length sub)))
    (let loop ((i 0))
      (cond
       ((> (+ i n) (string-length s)) #f)
       ((string=? (substring s i (+ i n)) sub) i)
       (else (loop (+ i 1)))))))

;; A tag-aware colon finder: the first colon outside quotes and tags, or #f
(define (find-key-colon s)
  (let loop ((i 0) (quote-char #f))
    (if (>= i (string-length s))
        #f
        (let ((c (string-ref s i)))
          (cond
           (quote-char
            (cond
             ((char=? c #\\) (loop (+ i 2) quote-char))
             ((char=? c quote-char) (loop (+ i 1) #f))
             (else (loop (+ i 1) quote-char))))
           ((or (char=? c #\") (char=? c #\')) (loop (+ i 1) c))
           ((char=? c #\() (loop (skip-tag s i) #f))
           ((skip-reference s i) => (lambda (end) (loop end #f)))
           ((char=? c #\:) i)
           (else (loop (+ i 1) #f)))))))

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
  ;; Split into lines. While inside a block string, opener is the column
  ;; that ends it: the indent of a bare opener, or the key's column for a
  ;; property opener. Its content lines are literal: no list leaders, no "-"
  ;; or "*" checks, no comments.
  (let ((lines-raw (string-split-newlines source)))
    (let loop ((lines lines-raw) (linenum 0) (acc '()) (opener -1))
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
              (cond
               ;; A line inside a block string
               ((and (>= opener 0) (or (= len 0) (> indent opener)))
                (loop (cdr lines) (+ linenum 1)
                      (cons (list rest indent "" linenum) acc)
                      opener))
               ;; Skip comment-only lines at column 0
               ((and (string-starts-with? rest "#") (= indent 0))
                (loop (cdr lines) (+ linenum 1) acc -1))
               (else
                  ;; Determine leader
                  (let* ((leader-and-content
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
                          (else (cons "" rest))))
                         (content (cdr leader-and-content)))
                    (loop (cdr lines) (+ linenum 1)
                          (cons (list content
                                      indent 
                                      (car leader-and-content) 
                                      linenum) 
                                acc)
                          (cond
                           ((or (bare-block-opener? content)
                                (bare-block-opener? (skip-prefix-tags content)))
                            indent)
                           ((or (string-ends-with? content ": `")
                                (tagged-property-opener? content))
                            (+ indent (- (string-length rest) (string-length content))))
                           (else -1))))))))))))

;; Whether a line's content (after any list leader) is a bare "`" or "` text"
;; block string opener.
(define (bare-block-opener? content)
  (or (string=? content "`")
      (string-starts-with? content "` ")))

;; Tokenizer: scanned lines -> tokens
(define (tokenize lines)
  (let ((tokens '())
        (stack (list 0))
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
         ;; Close blocks on dedent. A blank line has no indent of its own, so
         ;; it closes nothing.
         (let close-loop ()
           (when (and (> (string-length line) 0) (< indent (car stack)))
             (set! tokens (cons (list 'stop "") tokens))
             (set! stack (cdr stack))
             (close-loop)))
         (let ((top (car stack)))
           ;; Handle leader
           (cond
            ((and (> (string-length leader) 0) (> indent top))
             (set! tokens (cons (list 'start leader indent linenum) tokens))
             (set! stack (cons indent stack)))
            ((and (> (string-length leader) 0) (= indent top))
             (set! tokens (cons (list 'stop "") tokens))
             (set! tokens (cons (list 'start leader indent linenum) tokens))))
           ;; Handle line content. Every blank line is a break, so that a block
           ;; string keeps a run of blank lines.
           (if (> (string-length line) 0)
               (set! tokens (cons (list 'text line indent linenum) tokens))
               (set! tokens (cons (list 'break "" linenum) tokens))))))
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
        (unless (char=? (string-ref s (- len 1)) #\')
          (yay-error "Unterminated string"))
        ;; Handle \' escapes
        (let loop ((i 0) (acc '()))
          (if (>= i (string-length inner))
              (list->string (reverse acc))
              (let ((ch (string-ref inner i)))
                (cond
                 ((and (char=? ch #\\)
                       (< (+ i 1) (string-length inner))
                       (char=? (string-ref inner (+ i 1)) #\'))
                  (loop (+ i 2) (cons #\' acc)))
                 ;; The string ends here, and something follows it
                 ((char=? ch #\')
                  (yay-error "Unexpected character after string"))
                 (else
                  (loop (+ i 1) (cons ch acc))))))))))

;; Parse hex bytes - returns a tagged bytevector
(define (parse-hex-bytes hex-str)
  (let* ((bad #f)
         (hex (let loop ((i 0) (acc '()))
                (if (>= i (string-length hex-str))
                    (list->string (reverse acc))
                    (let ((c (string-ref hex-str i)))
                      (if (or (char=? c #\space) (char=? c #\newline))
                          (loop (+ i 1) acc)
                          (begin
                            ;; Check for uppercase hex digit
                            (when (uppercase-hex? c)
                              (yay-error "Uppercase hex digit (use lowercase)"))
                            ;; Count a non-hex digit; refuse it after the
                            ;; length, as the Rust port does
                            (when (not (hex-digit? c))
                              (set! bad #t))
                            (loop (+ i 1) (cons c acc))))))))
         (len (string-length hex)))
    (when (odd? len)
      (yay-error "Odd number of hex digits in byte literal"))
    (when bad
      (yay-error "Invalid hex digit"))
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
    ;; A "#" inside comments out the ">", as the Rust port reads it
    (when (string-index s #\#)
      (yay-error "Unmatched angle bracket"))
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
           ((and (not in-string) (skip-reference s i))
            => (lambda (end) (loop end #f #f)))
           ((and (not in-string) (char=? ch #\())
            (loop (+ i 1) #t #\)))
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
              (when (and (< (+ i 1) len)
                         (char=? (string-ref s (+ i 1)) #\:))
                (yay-error "Unexpected character \":\""))
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
                (loop end-i items (string-append current (substring s i end-i)))))
             ;; Start of string
             ((or (char=? ch #\") (char=? ch #\'))
              (receive (str end-i) (parse-inline-string s i ch)
                (loop end-i items (string-append current str))))
             ;; A reference, read whole
             ((skip-reference s i)
              => (lambda (end-i)
                   (loop end-i items (string-append current (substring s i end-i)))))
             ;; Start of bytes
             ((char=? ch #\<)
              (receive (bytes end-i) (parse-inline-bytes s i)
                (loop end-i items (string-append current (substring s i end-i)))))
             ;; A tag, read with the value before it
             ((char=? ch #\()
              (let ((end-i (skip-tag s i)))
                (loop end-i items (string-append current (substring s i end-i)))))
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
           ((char=? ch #\()
            (let ((end-i (skip-tag s i)))
              (loop end-i depth (string-append content (substring s i end-i)))))
           ((skip-reference s i)
            => (lambda (end-i)
                 (loop end-i depth (string-append content (substring s i end-i)))))
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
                (loop end-i pairs current-key (string-append current (substring s i end-i)))))
             ;; Start of nested array
             ((char=? ch #\[)
              (receive (nested end-i) (parse-nested-inline-array s i)
                (loop end-i pairs current-key (string-append current (substring s i end-i)))))
             ;; A tag, read with the value before it
             ((char=? ch #\()
              (let ((end-i (skip-tag s i)))
                (loop end-i pairs current-key (string-append current (substring s i end-i)))))
             ;; A reference, read whole
             ((skip-reference s i)
              => (lambda (end-i)
                   (loop end-i pairs current-key (string-append current (substring s i end-i)))))
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
           ((char=? ch #\()
            (let ((end-i (skip-tag s i)))
              (loop end-i depth (string-append content (substring s i end-i)))))
           ((skip-reference s i)
            => (lambda (end-i)
                 (loop end-i depth (string-append content (substring s i end-i)))))
           ((or (char=? ch #\") (char=? ch #\'))
            (receive (str end-i) (parse-inline-string s i ch)
              (loop end-i depth (string-append content str))))
           (else
            (loop (+ i 1) depth (string-append content (string ch)))))))))

;; Parse inline value (for array elements)
(define (parse-inline-value s)
  (parse-tagged parse-inline-atom (string-trim s)))

(define (parse-inline-atom s)
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
     ((reference-start? s 0) (parse-whole-reference s))
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
;; Index of the first colon outside quotes, or #f
(define (find-colon-outside-quotes s)
  (let loop ((i 0) (in-double #f) (in-single #f) (escaped #f))
    (if (>= i (string-length s))
        #f
        (let ((c (string-ref s i)))
          (cond
           (escaped (loop (+ i 1) in-double in-single #f))
           ((and in-double (char=? c #\\)) (loop (+ i 1) in-double in-single #t))
           ((and (char=? c #\() (not in-double) (not in-single))
            (loop (skip-tag s i) in-double in-single #f))
           ((and (not in-double) (not in-single) (skip-reference s i))
            => (lambda (end) (loop end in-double in-single #f)))
           ((and (char=? c #\") (not in-single)) (loop (+ i 1) (not in-double) in-single #f))
           ((and (char=? c #\') (not in-double)) (loop (+ i 1) in-double (not in-single) #f))
           ((and (char=? c #\:) (not in-double) (not in-single)) i)
           (else (loop (+ i 1) in-double in-single #f)))))))

(define (split-key-value s)
  ;; A reference is never a key: the colon after it is stray
  (when (reference-start? s 0)
    (yay-error "Unexpected character \":\""))
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
              (receive (tags rest)
                  (if (string-starts-with? value-part "(")
                      (parse-prefix value-part)
                      (values '() value-part))
                (when (and (pair? tags)
                           (not (string=? rest ""))
                           (not (string=? rest "`"))
                           (not (and (string-starts-with? rest ">")
                                     (not (string-contains? rest "<")))))
                  (yay-error prefix-without-block))
                (list key rest tags))))))))

;; Parse scalar value
;; The text before an inline comment, " #" outside quotes, without the
;; spaces before it
(define (strip-inline-comment s)
  (let loop ((i 0) (quote-char #f))
    (if (>= i (string-length s))
        s
        (let ((c (string-ref s i)))
          (cond
           (quote-char
            (cond
             ((char=? c #\\) (loop (+ i 2) quote-char))
             ((char=? c quote-char) (loop (+ i 1) #f))
             (else (loop (+ i 1) quote-char))))
           ((or (char=? c #\") (char=? c #\')) (loop (+ i 1) c))
           ;; A tag's text is not a comment
           ((char=? c #\() (loop (skip-tag s i) #f))
           ;; Nor is a reference's
           ((skip-reference s i) => (lambda (end) (loop end #f)))
           ((and (char=? c #\#) (> i 0) (char=? (string-ref s (- i 1)) #\space))
            (let trim ((j (- i 1)))
              (if (and (> j 0) (char=? (string-ref s (- j 1)) #\space))
                  (trim (- j 1))
                  (substring s 0 j))))
           (else (loop (+ i 1) #f)))))))

;; Parse a scalar value and any inline comment after it
(define (parse-scalar s)
  (parse-tagged parse-scalar-atom (strip-inline-comment s)))

(define (parse-scalar-atom s)
  (cond
   ((string=? s "null") 'null)
   ((string=? s "true") #t)
   ((string=? s "false") #f)
   ((string=? s "nan") +nan.0)
   ((string=? s "infinity") +inf.0)
   ((string=? s "-infinity") -inf.0)
   ((string-starts-with? s "\"") (parse-json-string s))
   ((string-starts-with? s "'") (parse-single-quoted-string s))
   ((reference-start? s 0) (parse-whole-reference s))
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
                        (if (string-starts-with? text "(")
                            ;; A tagged block: below the item, or a block
                            ;; string or block bytes on its line
                            (let ((value (parse-tagged-item p next list-indent)))
                              (skip-stops-and-breaks! p)
                              (loop (cons value items)))
                        (if (and (find-colon-outside-quotes text)
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
                              (loop (cons value items)))))))
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
  (let loop ((lines '()))
    (let ((t (parser-peek p)))
      (cond
       ((not t)
        (finish-block-string lines first-line))
       ((eq? (token-type t) 'break)
        (parser-advance! p)
        (loop (cons (cons #f "") lines)))
       ((and (eq? (token-type t) 'text)
             (>= base-indent 0)
             (<= (token-indent t) base-indent))
        ;; Stop: line at or below base indent
        (finish-block-string lines first-line))
       ((eq? (token-type t) 'text)
        (parser-advance! p)
        (loop (cons (cons (token-indent t) (token-text t)) lines)))
       (else
        (finish-block-string lines first-line))))))

;; lines are a block string's continuation lines, last first, each
;; (indent . text) with #f for the indent of a blank line.
(define (finish-block-string lines first-line)
  (let* ((body-lines (strip-block-indent (reverse lines)))
         (trimmed (trim-blank-lines (if (string=? first-line "")
                                        body-lines
                                        (cons first-line body-lines))))
         (body (string-join trimmed "\n"))
         (leading-nl (and (string=? first-line "") (pair? trimmed)))
         (result (string-append (if leading-nl "\n" "")
                                body
                                (if (pair? trimmed) "\n" ""))))
    (if (string=? result "")
        (error "Empty block string not allowed (use \"\" or \"\\n\" explicitly)")
        result)))

;; Strip the least indentation of the content lines from a block string's
;; lines, each (indent . text) with #f for the indent of a blank line.
(define (strip-block-indent entries)
  (let ((min-indent (let loop ((lst entries) (min-i #f))
                      (cond
                       ((null? lst) (or min-i 0))
                       ((and (car (car lst))
                             (or (not min-i) (< (car (car lst)) min-i)))
                        (loop (cdr lst) (car (car lst))))
                       (else (loop (cdr lst) min-i))))))
    (map (lambda (entry)
           (if (car entry)
               (string-append (make-string (- (car entry) min-indent) #\space)
                              (cdr entry))
               ""))
         entries)))

;; Drop the blank lines from both ends of a list of lines.
(define (trim-blank-lines lines)
  (let ((drop-blanks (lambda (lst)
                       (let loop ((lst lst))
                         (if (and (pair? lst) (string=? (car lst) ""))
                             (loop (cdr lst))
                             lst)))))
    (reverse (drop-blanks (reverse (drop-blanks lines))))))

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
              ;; A tagged inline value: tags after it, and no property's
              ;; colon before them
              ((let ((tag-at (find-top-level-tag s)))
                 (and tag-at
                      (> tag-at 0)
                      (not (string-starts-with? s "`"))
                      (not (string-starts-with? s ">"))
                      (or (string-starts-with? s "[")
                          (string-starts-with? s "{")
                          (let ((colon (find-key-colon s)))
                            (or (not colon) (> colon tag-at))))))
               (parser-advance! p)
               (parse-scalar s))
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
                 ;; The block ends at a line indented no deeper than the
                 ;; opener's line.
                 (parse-block-string-with-indent p first-line (token-indent t))))
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
              ;; A reference
              ((reference-start? s 0)
               (parser-advance! p)
               (parse-whole-reference (strip-inline-comment s)))
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
                       (wrap-property (caddr kv)
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
                        ((and (string-starts-with? value-part "<")
                       (not (reference-start? value-part 0)))
                         (if (string-contains? value-part ">")
                             (begin
                               (parser-advance! p)
                               (list (cons key (parse-tagged parse-angle-bytes value-part))))
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
                         (list (cons key (parse-scalar value-part)))))))
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
  (let* ((trimmed (trim-blank-lines (strip-block-indent (reverse lines))))
         (result (string-append (string-join trimmed "\n")
                                (if (pair? trimmed) "\n" ""))))
    (if (string=? result "")
        (error "Empty block string not allowed (use \"\" or \"\\n\" explicitly)")
        result)))

;; Parse object block inside an array item
;; first-indent is the indent of the first key (right after -)
;; array-indent is the indent of the array item marker
;; We accept keys at first-indent on the item's own line, OR at
;; array-indent + 2. A later line at first-indent is the next key of an
;; enclosing object when the list sits at its key's indent
;; ("items:\n- a: 1\nnext: 1").
(define (parse-object-block-in-array p first-indent array-indent)
  ;; The tags before the last property's block, to wrap its value in
  (define pending '())
  (let ((sibling-indent (+ array-indent 2))
        (item-line (token-linenum (parser-peek p))))
    (let loop ((pairs0 '()))
      (let ((pairs (if (null? pending)
                       pairs0
                       (let ((tags pending))
                         (set! pending '())
                         (cons (cons (caar pairs0) (wrap-prefix (cdar pairs0) tags))
                               (cdr pairs0)))))
            (t (parser-peek p)))
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
         ;; Accept text at first-indent on the item's line, or sibling-indent
         ((and (eq? (token-type t) 'text)
               (or (and (= (token-indent t) first-indent)
                        (= (token-linenum t) item-line))
                   (= (token-indent t) sibling-indent)))
          (let* ((s (token-text t))
                 (current-indent (token-indent t))
                 (kv (split-key-value s)))
            (if kv
                (let ((key (car kv))
                      (value-part (cadr kv)))
                  (set! pending (caddr kv))
                  (cond
                   ;; Empty value - nested
                   ((string=? value-part "")
                    (parser-advance! p)
                    (parser-skip-breaks! p)
                    ;; Under tags, the block must be indented under the
                    ;; key, or be a list at its column: a sibling item or
                    ;; property after "- a: (t)" is not its block
                    (let ((next (parser-peek p))
                          (tagged (pair? pending)))
                      (cond
                       ((and next (eq? (token-type next) 'start)
                             (string=? (token-text next) "-")
                             (or (not tagged)
                                 (>= (token-indent next) sibling-indent)))
                        (let ((arr (parse-list-array-impl p (token-indent next))))
                          (loop (cons (cons key arr) pairs))))
                       ((and next (eq? (token-type next) 'text)
                             (> (token-indent next)
                                (if tagged sibling-indent current-indent)))
                        (let ((obj (parse-object-block p (token-indent next))))
                          (loop (cons (cons key obj) pairs))))
                       ;; Empty value is always an error
                       (tagged
                        (yay-error prefix-without-block))
                       (else
                        (yay-error "Expected value after property")))))
                   ;; Empty object
                   ((string=? value-part "{}")
                    (parser-advance! p)
                    (loop (cons (cons key '()) pairs)))
                   ;; Angle bytes (< must have closing >)
                   ((and (string-starts-with? value-part "<")
                       (not (reference-start? value-part 0)))
                    (if (string-contains? value-part ">")
                        (begin
                          (parser-advance! p)
                          (loop (cons (cons key (parse-tagged parse-angle-bytes value-part)) pairs)))
                        (yay-error "Unmatched angle bracket")))
                   ;; Block string (backtick). The block ends at the key's
                   ;; column, which for "- key: `" is past the list leader.
                   ((or (string=? value-part "`")
                        (and (string-starts-with? value-part "`")
                             (>= (string-length value-part) 2)
                             (char=? (string-ref value-part 1) #\space)))
                    (parser-advance! p)
                    (let ((first-line (if (> (string-length value-part) 2)
                                          (substring value-part 2 (string-length value-part))
                                          "")))
                      (let ((str (parse-block-string-for-property p first-line sibling-indent)))
                        (loop (cons (cons key str) pairs)))))
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
  ;; The tags before the last property's block, to wrap its value in
  (define pending '())
  (let loop ((pairs0 '()))
    (let ((pairs (if (null? pending)
                     pairs0
                     (let ((tags pending))
                       (set! pending '())
                       (cons (cons (caar pairs0) (wrap-prefix (cdar pairs0) tags))
                             (cdr pairs0)))))
          (t (parser-peek p)))
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
                (set! pending (caddr kv))
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
                      ;; Check for concatenated quoted strings first. A quoted
                      ;; line with a colon outside its quotes is a property with
                      ;; a quoted key, which opens a nested object.
                      (let ((trimmed (string-trim (token-text next))))
                        (if (and (>= (string-length trimmed) 2)
                                 (or (and (char=? (string-ref trimmed 0) #\")
                                          (char=? (string-ref trimmed (- (string-length trimmed) 1)) #\"))
                                     (and (char=? (string-ref trimmed 0) #\')
                                          (char=? (string-ref trimmed (- (string-length trimmed) 1)) #\')))
                                 (not (find-colon-outside-quotes trimmed)))
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
                 ((and (string-starts-with? value-part "<")
                       (not (reference-start? value-part 0)))
                  (if (string-contains? value-part ">")
                      (begin
                        (parser-advance! p)
                        (loop (cons (cons key (parse-tagged parse-angle-bytes value-part)) pairs)))
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
       ;; Nothing may stand indented under a property's inline value
       ((and (memq (token-type t) '(text start))
             (> (token-indent t) base-indent))
        (yay-error "Unexpected indent"))
       (else
        (parser-advance! p)
        (loop pairs))))))

;; Wrap a property's value, (list (cons key value)), in its prefix's tags
(define (wrap-property tags result)
  (if (null? tags)
      result
      (list (cons (caar result) (wrap-prefix (cdar result) tags)))))

(define (skip-stops-and-breaks! p)
  (let ((t (parser-peek p)))
    (when (and t (or (eq? (token-type t) 'stop) (eq? (token-type t) 'break)))
      (parser-advance! p)
      (skip-stops-and-breaks! p))))

;; A list item that begins with tags: a block below the item, or a block
;; string or block bytes on the item's line
(define (parse-tagged-item p next list-indent)
  (receive (tags rest) (parse-prefix (token-text next))
    (cond
     ((bare-block-opener? rest)
      (let ((first-line (if (> (string-length rest) 2)
                            (substring rest 2 (string-length rest))
                            "")))
        (wrap-prefix (parse-block-string-with-indent p first-line (token-indent next))
                     tags)))
     ((and (string-starts-with? rest ">") (not (string-contains? rest "<")))
      (wrap-prefix (parse-block-bytes p rest) tags))
     ((string=? rest "")
      (parser-advance! p)
      (skip-stops-and-breaks! p)
      (let ((below (parser-peek p)))
        (cond
         ((or (not below) (<= (token-indent below) list-indent))
          (yay-error prefix-without-block))
         ((and (eq? (token-type below) 'start) (string=? (token-text below) "-"))
          (wrap-prefix (parse-list-array-impl p (token-indent below)) tags))
         ((and (eq? (token-type below) 'text) (find-key-colon (token-text below)))
          (wrap-prefix (parse-object-block p (token-indent below)) tags))
         ((and (eq? (token-type below) 'text)
               (or (string-starts-with? (token-text below) "\"")
                   (string-starts-with? (token-text below) "'")))
          (let ((result (parse-concatenated-strings p (token-indent below))))
            (if result
                (wrap-prefix result tags)
                (yay-error prefix-without-block))))
         (else (yay-error prefix-without-block)))))
     (else (yay-error prefix-without-block)))))

;; A tagged root block: the tags alone on the first line, the block below at
;; column 0
(define (parse-tagged-root p t)
  (receive (tags rest) (parse-prefix (token-text t))
    (unless (string=? rest "")
      (yay-error prefix-without-block))
    (parser-advance! p)
    (skip-stops-and-breaks! p)
    (let* ((below (parser-peek p))
           (value
            (cond
             ((or (not below) (not (= (token-indent below) 0)))
              (yay-error prefix-without-block))
             ((and (eq? (token-type below) 'start) (string=? (token-text below) "-"))
              (parse-list-array p))
             ((and (eq? (token-type below) 'text)
                   (not (string-starts-with? (token-text below) "{"))
                   (find-key-colon (token-text below)))
              (parse-object-block p 0))
             (else (yay-error prefix-without-block)))))
      (parser-skip-breaks! p)
      (skip-stops-and-breaks! p)
      (when (parser-peek p)
        (yay-error "Unexpected extra content"))
      (wrap-prefix value tags))))

;; Parse root
(define (parse-root p)
  (parser-skip-breaks! p)
  (let ((t (parser-peek p)))
    (if (not t)
        (yay-error "No value found in document")
        (if (and (eq? (token-type t) 'text)
                 (string-starts-with? (token-text t) "("))
            (parse-tagged-root p t)
        (let* ((is-root-object (and (eq? (token-type t) 'text)
                                    (find-key-colon (token-text t))
                                    (not (string-starts-with? (token-text t) "("))
                                    (= (token-indent t) 0)
                                    ;; Don't treat inline objects or arrays as root objects
                                    (not (string-starts-with? (token-text t) "{"))
                                    (not (string-starts-with? (token-text t) "["))))
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
          result)))))

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
   ((reference? value)
    (string-append "(make-reference " (format-scheme-string (reference-uri value)) ")"))
   ((tagged? value)
    (string-append "(make-tagged " (format-scheme-string (tagged-tag value))
                   " " (yay->scheme-string (tagged-payload value)) ")"))
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
   ;; Both references: equal by text; never equal to a string
   ((and (reference? a) (reference? b))
    (string=? (reference-uri a) (reference-uri b)))
   ;; Both tagged: equal tags and payloads; never equal to a payload
   ((and (tagged? a) (tagged? b))
    (and (string=? (tagged-tag a) (tagged-tag b))
         (yay-equal? (tagged-payload a) (tagged-payload b))))
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
