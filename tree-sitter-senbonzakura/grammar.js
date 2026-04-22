/// <reference types="tree-sitter-cli/dsl" />

module.exports = grammar({
  name: "senbonzakura",

  extras: ($) => [/[ \t\r\n]/, $.comment],

  word: ($) => $.identifier,

  conflicts: ($) => [],

  rules: {
    source_file: ($) => repeat($._statement),

    _statement: ($) =>
      choice(
        $.let_binding,
        $.mut_binding,
        $.function_def,
        $.data_decl,
        $.typeclass_decl,
        $.impl_block,
        $.if_statement,
        $.for_statement,
        $.while_statement,
        $.return_statement,
        $.import_statement,
        $.from_import_statement,
        $.assignment,
        $.expression_statement
      ),

    let_binding: ($) =>
      seq("let", field("name", $.identifier),
        optional(seq(":", field("type", $._type_expr))),
        "=", field("value", $._expression)),

    mut_binding: ($) =>
      seq("mut", field("name", $.identifier),
        optional(seq(":", field("type", $._type_expr))),
        "=", field("value", $._expression)),

    assignment: ($) =>
      prec(-1, seq(field("target", $._expression), "=", field("value", $._expression))),

    function_def: ($) =>
      seq("def", field("name", $.identifier),
        optional(field("type_params", $.type_param_list)),
        field("params", $.parameter_list),
        optional(seq("->", field("return_type", $._type_expr))),
        ":", field("body", $.block)),

    type_param_list: ($) => seq("[", commaSep1($.type_param), "]"),

    type_param: ($) =>
      seq(field("name", $.identifier),
        optional(seq(":", field("constraint", $.identifier)))),

    parameter_list: ($) => seq("(", commaSep($.parameter), ")"),

    parameter: ($) =>
      seq(field("name", $.identifier),
        optional(seq(":", field("type", $._type_expr)))),

    data_decl: ($) =>
      seq("data", field("name", $.identifier),
        optional(seq("[", commaSep1($.identifier), "]")),
        ":", field("body", $.data_body)),

    data_body: ($) => prec.right(repeat1($.field_decl)),

    field_decl: ($) =>
      seq(field("name", $.identifier), ":", field("type", $._type_expr)),

    typeclass_decl: ($) =>
      seq("typeclass", field("name", $.identifier),
        "[", commaSep1($.identifier), "]",
        ":", field("body", $.typeclass_body)),

    typeclass_body: ($) => prec.right(repeat1($.method_signature)),

    method_signature: ($) =>
      seq("def", field("name", $.identifier),
        field("params", $.parameter_list),
        "->", field("return_type", $._type_expr)),

    impl_block: ($) =>
      seq("impl", field("name", $.identifier),
        optional(seq("[", commaSep1($._type_expr), "]")),
        ":", field("body", $.impl_body)),

    impl_body: ($) => prec.right(repeat1($.function_def)),

    if_statement: ($) =>
      prec.right(seq("if", field("condition", $._expression), ":",
        field("then", $.block),
        repeat($.elif_clause),
        optional($.else_clause))),

    elif_clause: ($) =>
      seq("elif", field("condition", $._expression), ":", field("body", $.block)),

    else_clause: ($) =>
      seq("else", ":", field("body", $.block)),

    for_statement: ($) =>
      prec.right(seq("for", field("item", $.identifier), "in", field("iterable", $._expression), ":",
        field("body", $.block))),

    while_statement: ($) =>
      prec.right(seq("while", field("condition", $._expression), ":",
        field("body", $.block))),

    return_statement: ($) =>
      prec.right(seq("return", optional(field("value", $._expression)))),

    import_statement: ($) =>
      seq("import", field("module", $.dotted_name),
        optional(seq("as", field("alias", $.identifier)))),

    from_import_statement: ($) =>
      seq("from", field("module", $.dotted_name),
        "import", commaSep1($.import_name)),

    import_name: ($) =>
      seq(field("name", $.identifier),
        optional(seq("as", field("alias", $.identifier)))),

    dotted_name: ($) => sep1($.identifier, "."),

    expression_statement: ($) => prec(-2, $._expression),

    _expression: ($) =>
      choice(
        $.binary_expression,
        $.unary_expression,
        $.call_expression,
        $.attribute_access,
        $.parenthesized_expression,
        $.primary_expression
      ),

    binary_expression: ($) =>
      choice(
        prec.left(1, seq($._expression, "or", $._expression)),
        prec.left(2, seq($._expression, "and", $._expression)),
        prec.left(4, seq($._expression, field("operator", choice("==", "!=", "<", ">", "<=", ">=")), $._expression)),
        prec.left(5, seq($._expression, field("operator", choice("+", "-")), $._expression)),
        prec.left(6, seq($._expression, field("operator", choice("*", "/")), $._expression)),
      ),

    unary_expression: ($) =>
      choice(
        prec(7, seq("-", $._expression)),
        prec(3, seq("not", $._expression)),
      ),

    call_expression: ($) =>
      prec(8, seq(field("callee", $._expression), "(", commaSep($._expression), ")")),

    attribute_access: ($) =>
      prec.left(8, seq(field("object", $._expression), ".", field("attribute", $.identifier))),

    parenthesized_expression: ($) => seq("(", $._expression, ")"),

    primary_expression: ($) =>
      choice($.integer, $.float, $.string, $.boolean, $.none, $.identifier),

    // No external scanner for INDENT/DEDENT — sufficient for highlighting.
    block: ($) => prec.right(repeat1($._statement)),

    _type_expr: ($) => choice($.type_name, $.parameterized_type),

    type_name: ($) => $.identifier,

    parameterized_type: ($) =>
      seq(field("name", $.identifier), "[", commaSep1($._type_expr), "]"),

    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,
    integer: ($) => /[0-9]+/,
    float: ($) => /[0-9]+\.[0-9]+/,
    string: ($) =>
      seq('"', repeat(choice(/[^"\\]+/, $.escape_sequence)), '"'),
    escape_sequence: ($) => token.immediate(/\\[nrt\\"]/),
    boolean: ($) => choice("True", "False"),
    none: ($) => "None",
    comment: ($) => token(seq("#", /[^\n]*/)),
  },
});

function commaSep(rule) {
  return optional(commaSep1(rule));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}

function sep1(rule, separator) {
  return seq(rule, repeat(seq(separator, rule)));
}
