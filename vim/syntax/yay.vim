" Vim syntax file
" Language:    YAY (Yet Another YAML)
" Maintainer:  Kris Kowal
" License:     Apache 2.0
" URL:         https://github.com/kriskowal/yay

if exists('b:current_syntax')
  finish
endif

" --- Keywords ---------------------------------------------------------------

syntax keyword yayNull    null
syntax keyword yayBoolean true false

" --- Special floats ---------------------------------------------------------

" Must come before number rules so the keywords win over bare-word errors.
syntax keyword yayFloat   nan infinity
syntax match   yayFloat   /\v-infinity/

" --- Numbers ----------------------------------------------------------------

" Integers: optional minus, digits, optional digit-grouping spaces.
" The space-grouping pattern: digit, space, digit sequences.
syntax match yayInteger /\v-?\d+(\s\d+)*/

" Floats: require a decimal point or exponent to distinguish from integers.
" Decimal floats (with dot):
syntax match yayFloat /\v-?\d*\.\d*(\s\d+)*(e[+-]?\d+)?/
" Exponent-only floats (no dot):
syntax match yayFloat /\v-?\d+(\s\d+)*e[+-]?\d+/

" --- Strings ----------------------------------------------------------------

" Double-quoted strings with escape sequences.
syntax region yayString start=/"/ skip=/\\\\\|\\"/ end=/"/ contains=yayEscape,yayUnicodeEscape oneline
syntax match  yayEscape        /\\["\\/bfnrt]/ contained
syntax match  yayUnicodeEscape /\\u{[0-9a-fA-F]\{1,6}}/ contained

" Single-quoted strings (literal, only \' and \\ are escapes).
syntax region yaySingleString start=/'/ skip=/\\\\\|\\'/ end=/'/ oneline

" --- Block strings ----------------------------------------------------------

" Block string introducer: backtick at start of content (after indent),
" optionally followed by a space and same-line content.
" At root or array-item level:
syntax region yayBlockString matchgroup=yayBlockStringDelim start=/\v^(\s*)%(- )?`%( |$)/ end=/\v^%(\s*$|\1\S)/ contains=yayBlockStringBody keepend

" As a property value (key: ` must be alone):
syntax region yayBlockStringProp matchgroup=yayBlockStringDelim start=/\v(:\s+)`$/ end=/\v^%(\s*$|\s{0,}\S)/ contains=yayBlockStringBody keepend

" The body content itself (no special highlighting — just literal text).
syntax match yayBlockStringBody /.*/ contained

" --- Byte arrays ------------------------------------------------------------

" Inline byte arrays: <hex>
syntax region yayBytes start=/</ end=/>/ contains=yayHexContent oneline
syntax match  yayHexContent /[0-9a-f ]\+/ contained

" --- References -------------------------------------------------------------

" A reference: <scheme:...>, its text to the next >; a # inside is not a
" comment. Defined after bytes, so it wins where both start at a <.
syntax region yayReference matchgroup=yayReferenceDelim start=/<\ze[a-z][a-z0-9+.-]*:/ end=/>/ oneline

" Block byte array introducer and content.
" The > leader and subsequent indented hex lines are handled via match rules
" since region-based indent tracking is fragile.
syntax match yayBlockBytesLeader /\v^(\s*)%(- )?\>\s/
syntax match yayBlockBytesLeader /\v(:\s+)\>%(\s|$)/
syntax match yayHexLine          /\v^\s+[0-9a-f][0-9a-f \t]*%(\s+#.*)?$/ contains=yayHexData,yayComment
syntax match yayHexData          /\v[0-9a-f][0-9a-f \t]*[0-9a-f]/ contained
syntax match yayHexData          /\v[0-9a-f]{2}/ contained

" --- Tags -------------------------------------------------------------------

" A tag: (name), before a block or after a value; \) and \\ are its escapes.
syntax region yayTag matchgroup=yayTagDelim start=/(/ skip=/\\\\\|\\)/ end=/)/ contains=yayTagEscape oneline
syntax match  yayTagEscape /\\[\\)]/ contained

" --- Arrays -----------------------------------------------------------------

" Inline arrays.
syntax region yayInlineArray start=/\[/ end=/\]/ contains=yayString,yaySingleString,yayNull,yayBoolean,yayFloat,yayInteger,yayBytes,yayReference,yayTag,yayInlineArray,yayInlineObject,yayComment transparent

" --- Objects ----------------------------------------------------------------

" Inline objects.
syntax region yayInlineObject start=/{/ end=/}/ contains=yayString,yaySingleString,yayNull,yayBoolean,yayFloat,yayInteger,yayBytes,yayReference,yayTag,yayInlineArray,yayInlineObject,yayKey,yayComment transparent

" --- Keys -------------------------------------------------------------------

" Bare key followed by colon.
syntax match yayKey /\v[a-zA-Z0-9_-]+\ze:/ nextgroup=yayColon

" Double-quoted key followed by colon.
syntax match yayKey /\v"[^"]*"\ze:/ contains=yayEscape,yayUnicodeEscape nextgroup=yayColon

" Single-quoted key followed by colon.
syntax match yayKey /\v'[^']*'\ze:/ nextgroup=yayColon

" The colon separator.
syntax match yayColon /:/ contained

" --- List markers -----------------------------------------------------------

syntax match yayListMarker /\v^(\s*)- / contains=yayDash
syntax match yayDash       /\v-\ze / contained

" --- Comments ---------------------------------------------------------------

syntax match yayComment /\v#.*$/ contains=yayTodo
syntax keyword yayTodo TODO FIXME XXX NOTE HACK contained

" --- Highlight links --------------------------------------------------------

highlight default link yayNull          Constant
highlight default link yayBoolean       Boolean
highlight default link yayInteger       Number
highlight default link yayFloat         Float
highlight default link yayString        String
highlight default link yaySingleString  String
highlight default link yayEscape        SpecialChar
highlight default link yayUnicodeEscape SpecialChar
highlight default link yayBlockStringDelim Delimiter
highlight default link yayBlockStringBody String
highlight default link yayBytes         Special
highlight default link yayHexContent    Number
highlight default link yayBlockBytesLeader Delimiter
highlight default link yayHexLine       Special
highlight default link yayHexData       Number
highlight default link yayReference     Underlined
highlight default link yayReferenceDelim Delimiter
highlight default link yayKey           Identifier
highlight default link yayColon         Delimiter
highlight default link yayDash          Delimiter
highlight default link yayListMarker    Delimiter
highlight default link yayTag           Type
highlight default link yayTagDelim      Delimiter
highlight default link yayTagEscape     SpecialChar
highlight default link yayComment       Comment
highlight default link yayTodo          Todo

let b:current_syntax = 'yay'
