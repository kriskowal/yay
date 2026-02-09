;;; YAY Parser for Scheme (R7RS)
;;; Parses YAY documents and returns Scheme values

(define-library (yay)
  (export parse-yay yay->scheme-string)
  (import (scheme base)
          (scheme char)
          (scheme write))
  (begin

    ;; Error handling
    (define (yay-error msg . args)
      (error (apply string-append msg (map (lambda (x) (if (string? x) x (number->string x))) args))))

    ;; Character predicates
    (define (digit? c)
      (and (char? c) (char>=? c #\0) (char<=? c #\9)))

    (define (alpha? c)
      (and (char? c)
           (or (and (char>=? c #\a) (char<=? c #\z))
               (and (char>=? c #\A) (char<=? c #\Z)))))

    (define (alphanumeric? c)
      (or (alpha? c) (digit? c)))

    (define (hex-digit? c)
      (and (char? c)
           (or (digit? c)
               (and (char>=? c #\a) (char<=? c #\f)))))

    (define (hex-value c)
      (cond ((digit? c) (- (char->integer c) (char->integer #\0)))
            ((and (char>=? c #\a) (char<=? c #\f))
             (+ 10 (- (char->integer c) (char->integer #\a))))
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

    (define (string-trim-start s)
      (let loop ((i 0))
        (if (>= i (string-length s))
            ""
            (if (char=? (string-ref s i) #\space)
                (loop (+ i 1))
                (substring s i (string-length s))))))

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

    (define (string-split s delim)
      (let ((len (string-length s)))
        (let loop ((start 0) (i 0) (result '()))
          (cond ((>= i len)
                 (reverse (cons (substring s start len) result)))
                ((char=? (string-ref s i) delim)
                 (loop (+ i 1) (+ i 1) (cons (substring s start i) result)))
                (else (loop start (+ i 1) result))))))

    (define (string-join lst sep)
      (if (null? lst)
          ""
          (let loop ((rest (cdr lst)) (result (car lst)))
            (if (null? rest)
                result
                (loop (cdr rest) (string-append result sep (car rest)))))))

    ;; Count leading spaces
    (define (count-leading-spaces s)
      (let loop ((i 0))
        (if (or (>= i (string-length s))
                (not (char=? (string-ref s i) #\space)))
            i
            (loop (+ i 1)))))

    ;; Scanner: source -> list of (line indent leader linenum)
    (define (scan source)
      ;; Check for BOM
      (when (and (> (string-length source) 0)
                 (char=? (string-ref source 0) #\xFEFF))
        (yay-error "Illegal BOM"))
      ;; Check for surrogates (in UTF-8 string, check for isolated surrogates)
      ;; Split into lines
      (let* ((lines-raw (string-split source #\newline))
             (result '()))
        (let loop ((lines lines-raw) (linenum 0) (acc '()))
          (if (null? lines)
              (reverse acc)
              (let* ((line-str (car lines))
                     (len (string-length line-str)))
                ;; Check for tabs
                (let check-tab ((i 0))
                  (when (< i len)
                    (when (char=? (string-ref line-str i) #\tab)
                      (yay-error "Tab not allowed (use spaces)"))
                    (check-tab (+ i 1))))
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
                      (let-values (((leader line-content)
                                    (cond
                                     ;; "- " prefix
                                     ((string-starts-with? rest "- ")
                                      (values "-" (substring rest 2 (string-length rest))))
                                     ;; Bare "-"
                                     ((string=? rest "-")
                                      (values "-" ""))
                                     ;; "-" followed by digit or dot (negative number)
                                     ((and (> (string-length rest) 1)
                                           (char=? (string-ref rest 0) #\-)
                                           (or (digit? (string-ref rest 1))
                                               (char=? (string-ref rest 1) #\.)))
                                      (values "" rest))
                                     ;; "-" followed by non-space, non-dot, non-digit (list item)
                                     ((and (>= (string-length rest) 2)
                                           (char=? (string-ref rest 0) #\-)
                                           (not (char=? (string-ref rest 1) #\space))
                                           (not (char=? (string-ref rest 1) #\.))
                                           (not (digit? (string-ref rest 1))))
                                      (values "-" (substring rest 1 (string-length rest))))
                                     ;; Asterisk alone or with space (error in YAY)
                                     ((or (string=? rest "*")
                                          (and (>= (string-length rest) 2)
                                               (char=? (string-ref rest 0) #\*)
                                               (char=? (string-ref rest 1) #\space)))
                                      (yay-error "Unexpected character \"*\""))
                                     (else (values "" rest)))))
                        (loop (cdr lines) (+ linenum 1)
                              (cons (list line-content indent leader linenum) acc))))))))))

    ;; Tokenizer: scanned lines -> tokens
    (define (tokenize lines)
      (let ((tokens '())
            (stack (list 0))
            (broken #f))
        (for-each
         (lambda (entry)
           (let* ((line (car entry))
                  (indent (cadr entry))
                  (leader (caddr entry))
                  (linenum (cadddr entry))
                  (top (car stack)))
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
    (define (token-type t) (car t))
    (define (token-text t) (cadr t))
    (define (token-indent t) (if (>= (length t) 3) (caddr t) 0))
    (define (token-linenum t) (if (>= (length t) 4) (cadddr t) 0))

    ;; Parse number (returns #f if not a number)
    (define (parse-number s)
      (let* ((compact (let loop ((i 0) (acc '()))
                        (if (>= i (string-length s))
                            (list->string (reverse acc))
                            (let ((c (string-ref s i)))
                              (if (char=? c #\space)
                                  (loop (+ i 1) acc)
                                  (loop (+ i 1) (cons c acc)))))))
             (len (string-length compact)))
        (cond
         ((= len 0) #f)
         ;; Integer: optional minus followed by digits
         ((let ((start (if (and (> len 0) (char=? (string-ref compact 0) #\-)) 1 0)))
            (and (< start len)
                 (let loop ((i start))
                   (or (>= i len)
                       (and (digit? (string-ref compact i))
                            (loop (+ i 1)))))))
          ;; It's an integer
          (string->number compact))
         ;; Float: has a decimal point
         ((string-index compact #\.)
          (let* ((dot-pos (string-index compact #\.))
                 (before (substring compact 0 dot-pos))
                 (after (substring compact (+ dot-pos 1) len))
                 (has-minus (and (> (string-length before) 0)
                                 (char=? (string-ref before 0) #\-)))
                 (before-digits (if has-minus
                                    (substring before 1 (string-length before))
                                    before)))
            ;; Validate: before and after should be all digits (possibly empty)
            (if (and (let loop ((i 0))
                       (or (>= i (string-length before-digits))
                           (and (digit? (string-ref before-digits i))
                                (loop (+ i 1)))))
                     (let loop ((i 0))
                       (or (>= i (string-length after))
                           (and (digit? (string-ref after i))
                                (loop (+ i 1))))))
                ;; Valid float
                (let ((num-str (if (string=? before-digits "")
                                   (string-append (if has-minus "-0" "0") "." after)
                                   (if (string=? after "")
                                       (string-append compact "0")
                                       compact))))
                  (string->number num-str))
                #f)))
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
                                ((#\f) (loop (+ i 2) (cons #\x0C acc)))
                                ((#\n) (loop (+ i 2) (cons #\newline acc)))
                                ((#\r) (loop (+ i 2) (cons #\return acc)))
                                ((#\t) (loop (+ i 2) (cons #\tab acc)))
                                ((#\u)
                                 (if (< (+ i 6) len)
                                     (let* ((hex-str (substring s (+ i 2) (+ i 6)))
                                            (valid (let check ((j 0))
                                                     (or (>= j 4)
                                                         (and (let ((c (string-ref hex-str j)))
                                                                (or (digit? c)
                                                                    (and (char>=? c #\a) (char<=? c #\f))
                                                                    (and (char>=? c #\A) (char<=? c #\F))))
                                                              (check (+ j 1)))))))
                                       (if (not valid)
                                           (yay-error "Bad Unicode escape")
                                           (let ((code (string->number hex-str 16)))
                                             (if (and (>= code #xD800) (<= code #xDFFF))
                                                 (yay-error "Illegal surrogate")
                                                 (loop (+ i 6) (cons (integer->char code) acc))))))
                                     (yay-error "Bad Unicode escape")))
                                (else (yay-error "Bad escaped character")))))
                        ;; Regular character
                        (begin
                          (when (< (char->integer ch) #x20)
                            (yay-error "Bad character in string"))
                          (loop (+ i 1) (cons ch acc))))))))))

    ;; Parse single-quoted string (no escapes except \')
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

    ;; Parse quoted string (double or single)
    (define (parse-quoted-string s)
      (cond
       ((string-starts-with? s "\"") (parse-json-string s))
       ((string-starts-with? s "'") (parse-single-quoted-string s))
       (else s)))

    ;; Parse hex bytes
    (define (parse-hex-bytes hex-str)
      (let* ((hex (let loop ((i 0) (acc '()))
                    (if (>= i (string-length hex-str))
                        (list->string (reverse acc))
                        (let ((c (string-ref hex-str i)))
                          (if (or (char=? c #\space) (char=? c #\newline))
                              (loop (+ i 1) acc)
                              (loop (+ i 1) (cons (char-downcase c) acc)))))))
             (len (string-length hex)))
        (when (odd? len)
          (yay-error "Odd number of hex digits in byte literal"))
        (let loop ((i 0) (bytes '()))
          (if (>= i len)
              (list->vector (reverse bytes))
              (let ((hi (hex-value (string-ref hex i)))
                    (lo (hex-value (string-ref hex (+ i 1)))))
                (loop (+ i 2) (cons (+ (* hi 16) lo) bytes)))))))

    ;; Parse angle-bracket bytes <...>
    (define (parse-angle-bytes s)
      (if (string=? s "<>")
          (vector)
          (let* ((inner (substring s 1 (- (string-length s) 1)))
                 (hex (let loop ((i 0) (acc '()))
                        (if (>= i (string-length inner))
                            (list->string (reverse acc))
                            (let ((c (string-ref inner i)))
                              (if (char-whitespace? c)
                                  (loop (+ i 1) acc)
                                  (loop (+ i 1) (cons (char-downcase c) acc))))))))
            (parse-hex-bytes hex))))

    ;; Parse inline array [...]
    (define (parse-inline-array s tokens-ref idx-ref)
      (let ((len (string-length s)))
        (when (not (string-contains? s "]"))
          (yay-error "Unexpected newline in inline array"))
        ;; Simple parser for inline arrays
        (let loop ((i 1) (items '()) (current ""))
          (if (>= i len)
              (list->vector (reverse items))
              (let ((ch (string-ref s i)))
                (cond
                 ;; End of array
                 ((char=? ch #\])
                  (let ((items (if (string=? current "")
                                   items
                                   (cons (parse-inline-value (string-trim current)) items))))
                    (list->vector (reverse items))))
                 ;; Comma separator
                 ((char=? ch #\,)
                  (loop (+ i 1)
                        (cons (parse-inline-value (string-trim current)) items)
                        ""))
                 ;; Start of nested array
                 ((char=? ch #\[)
                  (let-values (((nested end-i) (parse-nested-inline-array s i)))
                    (loop end-i (cons nested items) "")))
                 ;; Start of string
                 ((or (char=? ch #\") (char=? ch #\'))
                  (let-values (((str end-i) (parse-inline-string s i ch)))
                    (loop end-i items (string-append current str))))
                 ;; Start of bytes
                 ((char=? ch #\<)
                  (let-values (((bytes end-i) (parse-inline-bytes s i)))
                    (loop end-i (cons bytes items) "")))
                 ;; Regular character
                 (else
                  (loop (+ i 1) items (string-append current (string ch))))))))))

    ;; Parse nested inline array
    (define (parse-nested-inline-array s start)
      (let loop ((i (+ start 1)) (depth 1) (content "["))
        (if (or (>= i (string-length s)) (= depth 0))
            (values (parse-inline-array content #f #f) i)
            (let ((ch (string-ref s i)))
              (cond
               ((char=? ch #\[)
                (loop (+ i 1) (+ depth 1) (string-append content (string ch))))
               ((char=? ch #\])
                (if (= depth 1)
                    (values (parse-inline-array (string-append content "]") #f #f) (+ i 1))
                    (loop (+ i 1) (- depth 1) (string-append content (string ch)))))
               ((or (char=? ch #\") (char=? ch #\'))
                (let-values (((str end-i) (parse-inline-string s i ch)))
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

    ;; Trim whitespace from both ends
    (define (string-trim s)
      (let* ((len (string-length s))
             (start (let loop ((i 0))
                      (if (or (>= i len) (not (char-whitespace? (string-ref s i))))
                          i
                          (loop (+ i 1)))))
             (end (let loop ((i len))
                    (if (or (<= i start) (not (char-whitespace? (string-ref s (- i 1)))))
                        i
                        (loop (- i 1))))))
        (substring s start end)))

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
         ((string-starts-with? s "<") (parse-angle-bytes s))
         ((string-starts-with? s "[") (parse-inline-array s #f #f))
         (else
          (let ((num (parse-number s)))
            (if num num s))))))

    ;; Split key:value
    (define (split-key-value s)
      (let ((colon-idx (string-index s #\:)))
        (if (not colon-idx)
            #f
            (let* ((key-raw (substring s 0 colon-idx))
                   (value-slice (substring s (+ colon-idx 1) (string-length s))))
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
       ((string-starts-with? s "[") (parse-inline-array s #f #f))
       ((string-starts-with? s "{")
        (if (string=? s "{}")
            '()
            (yay-error "Unexpected inline object")))
       (else
        (let ((num (parse-number s)))
          (if num num s)))))

    ;; Parse list array (- items)
    (define (parse-list-array p)
      (let loop ((items '()))
        (let ((t (parser-peek p)))
          (if (and t (eq? (token-type t) 'start) (string=? (token-text t) "-"))
              (let ((list-indent (token-indent t)))
                (parser-advance! p)
                (parser-skip-breaks! p)
                (let ((next (parser-peek p)))
                  (cond
                   ;; Nested list
                   ((and next (eq? (token-type next) 'start) (string=? (token-text next) "-"))
                    (let ((nested (parse-list-array p)))
                      ;; Skip stops
                      (let skip-stops ()
                        (let ((t (parser-peek p)))
                          (when (and t (eq? (token-type t) 'stop))
                            (parser-advance! p)
                            (skip-stops))))
                      (loop (cons nested items))))
                   ;; Text value
                   ((and next (eq? (token-type next) 'text))
                    (let ((value (parse-value p)))
                      ;; Skip stops
                      (let skip-stops ()
                        (let ((t (parser-peek p)))
                          (when (and t (eq? (token-type t) 'stop))
                            (parser-advance! p)
                            (skip-stops))))
                      (loop (cons value items))))
                   (else
                    ;; Skip stops and continue
                    (let skip-stops ()
                      (let ((t (parser-peek p)))
                        (when (and t (eq? (token-type t) 'stop))
                          (parser-advance! p)
                          (skip-stops))))
                    (loop items)))))
              (list->vector (reverse items))))))

    ;; Parse block string
    (define (parse-block-string p first-line)
      (parser-advance! p)
      (let loop ((lines (if (string=? first-line "") '() (list first-line))))
        (let ((t (parser-peek p)))
          (cond
           ((not t)
            (finish-block-string lines first-line))
           ((eq? (token-type t) 'break)
            (parser-advance! p)
            (loop (cons "" lines)))
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
                                 (string=? (car (reverse lst)) ""))
                            (trim-end (reverse (cdr (reverse lst))))
                            lst)))
             ;; Trim leading empty lines
             (trimmed2 (let trim-start ((lst trimmed))
                         (if (and (pair? lst) (string=? (car lst) ""))
                             (trim-start (cdr lst))
                             lst)))
             (body (string-join trimmed2 "\n"))
             (leading-nl (and (string=? first-line "") (pair? trimmed2))))
        (string-append (if leading-nl "\n" "")
                       body
                       (if (pair? trimmed2) "\n" ""))))

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

    ;; Parse value
    (define (parse-value p)
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
                 (parse-value p))))
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
                  ;; Block string
                  ((or (string=? s "\"")
                       (and (string-starts-with? s "\"")
                            (>= (string-length s) 2)
                            (char=? (string-ref s 1) #\space)))
                   (let ((first-line (if (> (string-length s) 2)
                                         (substring s 2 (string-length s))
                                         "")))
                     (parse-block-string p first-line)))
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
                   (parse-inline-array s #f #f))
                  ;; Angle bytes (complete)
                  ((and (string-starts-with? s "<") (string-contains? s ">"))
                   (parser-advance! p)
                   (parse-angle-bytes s))
                  ;; Multiline angle bytes
                  ((string-starts-with? s "<")
                   (parse-multiline-angle-bytes p s (token-indent t)))
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
                                ;; Array
                                ((and next (eq? (token-type next) 'start)
                                      (string=? (token-text next) "-"))
                                 (let ((arr (parse-list-array p)))
                                   (list (cons key arr))))
                                ;; Nested object
                                ((and next (eq? (token-type next) 'text))
                                 (let ((obj (parse-object-block p (token-indent next))))
                                   (list (cons key obj))))
                                ;; Block string
                                ((and next (eq? (token-type next) 'text)
                                      (string=? (token-text next) "\""))
                                 (let ((str (parse-block-string p "")))
                                   (list (cons key str))))
                                (else
                                 (list (cons key 'null))))))
                            ;; Empty object
                            ((string=? value-part "{}")
                             (parser-advance! p)
                             (list (cons key '())))
                            ;; Multiline bytes
                            ((and (string-starts-with? value-part "<")
                                  (not (string-contains? value-part ">")))
                             (let ((bytes (parse-multiline-angle-bytes p value-part (token-indent t))))
                               (list (cons key bytes))))
                            ;; Block string as property
                            ((or (string=? value-part "\"")
                                 (and (string-starts-with? value-part "\"")
                                      (>= (string-length value-part) 2)
                                      (char=? (string-ref value-part 1) #\space)))
                             (parser-advance! p)
                             (let ((first-line (if (> (string-length value-part) 2)
                                                   (substring value-part 2 (string-length value-part))
                                                   "")))
                               (let ((str (parse-block-string-for-property p first-line (token-indent t))))
                                 (list (cons key str)))))
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
      (let loop ((lines (if (string=? first-line "") '() (list first-line))))
        (let ((t (parser-peek p)))
          (cond
           ((not t)
            (finish-block-string-property lines first-line))
           ((eq? (token-type t) 'break)
            (parser-advance! p)
            (loop (cons "" lines)))
           ((and (eq? (token-type t) 'text)
                 (> (token-indent t) base-indent))
            (parser-advance! p)
            (loop (cons (token-text t) lines)))
           (else
            (finish-block-string-property lines first-line))))))

    (define (finish-block-string-property lines first-line)
      (let* ((rev-lines (reverse lines))
             ;; Trim trailing empty lines
             (trimmed (let trim-end ((lst rev-lines))
                        (if (and (pair? lst)
                                 (string=? (car (reverse lst)) ""))
                            (trim-end (reverse (cdr (reverse lst))))
                            lst))))
        (string-append (string-join trimmed "\n")
                       (if (pair? trimmed) "\n" ""))))

    ;; Parse object block
    (define (parse-object-block p base-indent)
      (let loop ((pairs '()))
        (let ((t (parser-peek p)))
          (cond
           ((not t)
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
                          (let ((arr (parse-list-array p)))
                            (loop (cons (cons key arr) pairs))))
                         ((and next (eq? (token-type next) 'text)
                               (> (token-indent next) base-indent))
                          (let ((obj (parse-object-block p (token-indent next))))
                            (loop (cons (cons key obj) pairs))))
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
                               (string-starts-with? (token-text next) "<")
                               (not (string-contains? (token-text next) ">")))
                          (let ((bytes (parse-multiline-angle-bytes p (token-text next) base-indent)))
                            (loop (cons (cons key bytes) pairs))))
                         (else
                          (loop (cons (cons key 'null) pairs))))))
                     ;; Empty object
                     ((string=? value-part "{}")
                      (parser-advance! p)
                      (loop (cons (cons key '()) pairs)))
                     ;; Multiline bytes
                     ((and (string-starts-with? value-part "<")
                           (not (string-contains? value-part ">")))
                      (let ((bytes (parse-multiline-angle-bytes p value-part base-indent)))
                        (loop (cons (cons key bytes) pairs))))
                     ;; Block string
                     ((or (string=? value-part "\"")
                          (and (string-starts-with? value-part "\"")
                               (>= (string-length value-part) 2)
                               (char=? (string-ref value-part 1) #\space)))
                      (parser-advance! p)
                      (let ((first-line (if (> (string-length value-part) 2)
                                            (substring value-part 2 (string-length value-part))
                                            "")))
                        (let ((str (parse-block-string-for-property p first-line base-indent)))
                          (loop (cons (cons key str) pairs)))))
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
            'null
            (if (and (eq? (token-type t) 'text)
                     (string-index (token-text t) #\:)
                     (= (token-indent t) 0))
                ;; Root object
                (parse-object-block p 0)
                ;; Single value
                (parse-value p)))))

    ;; Main entry point
    (define (parse-yay source)
      (let* ((lines (scan source))
             (tokens (tokenize lines))
             (parser (make-parser tokens)))
        (parse-root parser)))

    ;; Convert YAY value to Scheme string representation
    (define (yay->scheme-string value)
      (cond
       ((eq? value 'null) "'null")
       ((eq? value #t) "#t")
       ((eq? value #f) "#f")
       ((and (number? value) (nan? value)) "+nan.0")
       ((and (number? value) (infinite? value))
        (if (positive? value) "+inf.0" "-inf.0"))
       ((and (number? value) (exact? value)) (number->string value))
       ((and (number? value) (inexact? value))
        (let ((s (number->string value)))
          ;; Ensure there's a decimal point
          (if (string-index s #\.)
              s
              (string-append s ".0"))))
       ((string? value) (format-scheme-string value))
       ((vector? value)
        (if (and (> (vector-length value) 0)
                 (number? (vector-ref value 0))
                 (exact? (vector-ref value 0))
                 (<= (vector-ref value 0) 255))
            ;; Byte array
            (format-bytevector value)
            ;; Regular array
            (string-append "#("
                           (string-join (map yay->scheme-string (vector->list value)) " ")
                           ")")))
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
       (else (format "~a" value))))

    ;; Format string for Scheme output
    (define (format-scheme-string s)
      (string-append "\""
                     (let loop ((i 0) (acc '()))
                       (if (>= i (string-length s))
                           (list->string (reverse acc))
                           (let ((c (string-ref s i)))
                             (cond
                              ((char=? c #\") (loop (+ i 1) (append '(#\" #\\) acc)))
                              ((char=? c #\\) (loop (+ i 1) (append '(#\\ #\\) acc)))
                              ((char=? c #\newline) (loop (+ i 1) (append '(#\n #\\) acc)))
                              ((char=? c #\return) (loop (+ i 1) (append '(#\r #\\) acc)))
                              ((char=? c #\tab) (loop (+ i 1) (append '(#\t #\\) acc)))
                              ((char=? c #\backspace) (loop (+ i 1) (append '(#\b #\\) acc)))
                              ((char=? c #\x0C) (loop (+ i 1) (append '(#\f #\\) acc)))
                              (else (loop (+ i 1) (cons c acc)))))))
                     "\""))

    ;; Format bytevector
    (define (format-bytevector v)
      (string-append "#vu8("
                     (string-join
                      (map (lambda (b)
                             (string-append "#x"
                                            (let ((s (number->string b 16)))
                                              (if (= (string-length s) 1)
                                                  (string-append "0" s)
                                                  s))))
                           (vector->list v))
                      " ")
                     ")"))

    ))
