; Keywords
"let" @keyword
"mut" @keyword
"def" @keyword
"data" @keyword
"typeclass" @keyword
"impl" @keyword
"if" @keyword
"elif" @keyword
"else" @keyword
"return" @keyword
"import" @keyword
"from" @keyword
"as" @keyword
"and" @keyword
"or" @keyword
"not" @keyword
(boolean) @constant.builtin
(none) @constant.builtin

; Literals
(integer) @constant.numeric
(float) @constant.numeric
(string) @string
(escape_sequence) @string.special
(comment) @comment

; Function definitions
(function_def name: (identifier) @function)
(method_signature name: (identifier) @function)

; Data/typeclass declarations
(data_decl name: (identifier) @type)
(typeclass_decl name: (identifier) @type)
(impl_block name: (identifier) @type)

; Parameters
(parameter name: (identifier) @variable.parameter)

; Field declarations
(field_decl name: (identifier) @variable.other.member)

; Variables
(let_binding name: (identifier) @variable)
(mut_binding name: (identifier) @variable)

; Operators
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"==" @operator
"!=" @operator
"<" @operator
">" @operator
"<=" @operator
">=" @operator
"=" @operator
"->" @operator

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
":" @punctuation.delimiter
"," @punctuation.delimiter
"." @punctuation.delimiter

; Fallback
(identifier) @variable
