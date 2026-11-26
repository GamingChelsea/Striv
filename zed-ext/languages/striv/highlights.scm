(line_comment) @comment
(block_comment) @comment

(string_literal) @string
(raw_string_literal) @string
(char_literal) @string
(escape_sequence) @string.escape

(integer_literal) @number
(float_literal) @number
(boolean_literal) @boolean

[
  "fn"
  "let"
  "mut"
  "const"
  "struct"
  "enum"
  "impl"
  "use"
  "if"
  "else"
  "for"
  "in"
  "while"
  "loop"
  "match"
  "return"
  "break"
  "continue"
  "as"
  "pub"
  "mod"
] @keyword

((identifier) @keyword
  (#any-of? @keyword "tweak" "query" "entity" "parallel" "shared" "lock" "select"))

(type_identifier) @type
(primitive_type) @type

((identifier) @type.builtin
  (#any-of? @type.builtin "int" "float" "bool" "str" "Ecs" "Vec" "Bytes"))

(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function.method))
(call_expression function: (scoped_identifier name: (identifier) @function))

(field_identifier) @property
(parameter pattern: (identifier) @variable.parameter)
(scoped_identifier path: (identifier) @type)

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  ";"
  ":"
  "::"
  "."
] @punctuation.delimiter

[
  "->"
  "="
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "+"
  "-"
  "*"
  "/"
  "&&"
  "||"
  "!"
  "+="
  "-="
] @operator
