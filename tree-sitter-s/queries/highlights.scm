(string) @string
(comment) @comment
(keyword) @property
(symbol) @variable
(float) @number
(integer) @number
(quote _+) @macro
(quasiquote _+) @macro


(list (symbol) @keyword _+
    (#any-of? @keyword
        "if"
        "let"
        "let*"
        "do"
        "fn"
        "error"
        "thunk"
        "module"
        "import"
        "ref"
        "mut"
        "refmut"
        "enum"
        "match"
        "type"
        "newtype"
    )
)

((symbol) @keyword
    (#any-of? @keyword
        "self"
        "super"
        "root"
        "origin"
    )
)

(boolean) @keyword

[
    ".."
    "'"
    "`"
    ","
] @operator

(list (symbol) @operator _+
    (#any-of? @operator
        "+"
        "-"
        "*"
    )
)

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket
