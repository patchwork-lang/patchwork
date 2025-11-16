// @ts-check
/* eslint-disable no-useless-escape */

const PREC = {
  assignment: 1,
  conditional: 2,
  logical_or: 3,
  logical_and: 4,
  equality: 5,
  relational: 6,
  additive: 7,
  multiplicative: 8,
  unary: 9,
  call: 10,
  member: 11,
};

module.exports = grammar({
  name: "patchwork",

  extras: ($) => [/[ \t\u00A0\f]/, $.comment],

  externals: ($) => [
    $.prompt_start,
    $.prompt_end,
    $.prompt_text,
    $.prompt_escape,
    $.prompt_interpolation_start,
    $.prompt_interpolation_end,
    $.prompt_do,
    $._statement_terminator,
  ],

  conflicts: ($) => [
    [$.call_expression, $.member_expression],
    [$.prompt_block, $.block],
    [$.return_statement, $.prompt_block],
    [$.object_literal, $._statement_separator],
    [$.block, $.object_literal],
    [$.expression, $.object_field],
  ],

  supertypes: ($) => [$.statement, $.expression, $.type_expression],

  inline: ($) => [$._declaration],

  rules: {
    source_file: ($) =>
      seq(
        optional($._statement_separator),
        optional(
          seq(
            $._item,
            repeat(seq($._statement_separator, $._item)),
            optional($._statement_separator),
          ),
        ),
      ),

    _item: ($) =>
      seq(
        repeat($.annotation),
        optional("export"),
        choice($._declaration, $.statement),
      ),

    _declaration: ($) =>
      choice(
        $.worker_declaration,
        $.trait_declaration,
        $.skill_declaration,
        $.task_declaration,
        $.function_declaration,
        $.type_declaration,
        $.import_statement,
      ),

    annotation: ($) =>
      prec.right(
        seq(
          "@",
          field("name", $.identifier),
          optional(field("argument", choice($.identifier, $.string))),
        ),
      ),

    import_statement: ($) =>
      seq(
        "import",
        field("clause", choice($.identifier, $.destructuring_pattern)),
        optional(seq("from", field("source", choice($.string, $.identifier)))),
      ),

    worker_declaration: ($) =>
      seq(
        "worker",
        field("name", $.identifier),
        field("parameters", optional($.parameter_list)),
        field("body", $.block),
      ),

    skill_declaration: ($) =>
      seq(
        "skill",
        field("name", $.identifier),
        field("parameters", optional($.parameter_list)),
        field("body", $.block),
      ),

    trait_declaration: ($) =>
      seq(
        "trait",
        field("name", $.identifier),
        optional(seq(":", field("base", $.type_expression))),
        field("body", $.trait_body),
      ),

    trait_body: ($) =>
      seq(
        "{",
        optional($._statement_separator),
        optional(
          seq(
            $._trait_member,
            repeat(seq($._statement_separator, $._trait_member)),
            optional($._statement_separator),
          ),
        ),
        "}",
      ),

    _trait_member: ($) =>
      seq(
        repeat(seq($.annotation, optional($._statement_separator))),
        $.function_declaration,
      ),

    task_declaration: ($) =>
      seq(
        "task",
        field("name", $.identifier),
        field("parameters", optional($.parameter_list)),
        field("body", $.block),
      ),

    function_declaration: ($) =>
      seq(
        "fun",
        field("name", $.identifier),
        field("parameters", optional($.parameter_list)),
        optional(seq(":", field("return_type", $.type_expression))),
        field("body", $.block),
      ),

    type_declaration: ($) =>
      seq(
        "type",
        field("name", $.identifier),
        "=",
        field("value", $.type_expression),
      ),

    statement: ($) =>
      choice(
        $.block,
        $.var_declaration,
        $.return_statement,
        $.break_statement,
        $.continue_statement,
        $.if_statement,
        $.while_statement,
        $.for_statement,
        $.expression_statement,
        $.shell_command_statement,
      ),

    block: ($) =>
      seq(
        "{",
        optional($._statement_separator),
        optional(
          seq(
            $._annotated_statement,
            repeat(seq($._statement_separator, $._annotated_statement)),
            optional($._statement_separator),
          ),
        ),
        "}",
      ),

    var_declaration: ($) =>
      seq(
        "var",
        field("name", $.identifier),
        optional(seq(":", field("type", $.type_expression))),
        optional(seq("=", field("value", $.expression))),
      ),

    return_statement: ($) => seq("return", optional($.expression)),

    break_statement: ($) => seq(choice("break", "succeed", "fail")),

    continue_statement: ($) => seq("continue"),

    if_statement: ($) =>
      seq(
        "if",
        field("condition", $.parenthesized_expression),
        field("consequence", $.block),
        optional(field("alternative", $.else_clause)),
      ),

    else_clause: ($) => seq("else", choice($.block, $.if_statement)),

    while_statement: ($) =>
      seq(
        "while",
        field("condition", $.parenthesized_expression),
        field("body", $.block),
      ),

    for_statement: ($) =>
      seq(
        "for",
        field("initializer", $.parenthesized_expression),
        field("body", $.block),
      ),

    expression_statement: ($) => $.expression,

    prompt_block: ($) =>
      seq(field("kind", choice("think", "ask")), field("body", $.prompt_body)),

    prompt_body: ($) =>
      seq(
        $.prompt_start,
        repeat(
          choice(
            $.prompt_text,
            $.prompt_escape,
            $.prompt_interpolation,
            $.prompt_do_block,
          ),
        ),
        $.prompt_end,
      ),

    prompt_interpolation: ($) =>
      choice(
        seq(
          alias($.prompt_interpolation_start, "${"),
          field("expression", $.expression),
          alias($.prompt_interpolation_end, "}"),
        ),
        seq("$", field("identifier", $.identifier)),
      ),

    prompt_do_block: ($) => seq($.prompt_do, $.block),

    shell_command_statement: ($) => seq("$", field("command", $.shell_text)),

    shell_text: (_) => token.immediate(/[^\n{}]+/),

    expression: ($) =>
      choice(
        $.assignment_expression,
        $.binary_expression,
        $.unary_expression,
        $.await_expression,
        $.call_expression,
        $.member_expression,
        $.shell_command_expression,
        $.prompt_block,
        $.parenthesized_expression,
        $.array_literal,
        $.object_literal,
        $.identifier,
        $.number,
        $.string,
        $.boolean,
        $.self_expression,
      ),

    shell_command_expression: ($) =>
      seq("$(", field("command", $.shell_inner_text), ")"),

    shell_inner_text: (_) => token.immediate(/[^)]+/),

    await_expression: ($) => seq("await", $.expression),

    assignment_expression: ($) =>
      prec.right(
        PREC.assignment,
        seq(
          field("left", choice($.identifier, $.member_expression)),
          "=",
          field("right", $.expression),
        ),
      ),

    binary_expression: ($) => {
      const table = [
        [PREC.logical_or, "||"],
        [PREC.logical_and, "&&"],
        [PREC.equality, choice("==", "!=")],
        [PREC.relational, choice("<", "<=", ">", ">=")],
        [PREC.additive, choice("+", "-")],
        [PREC.multiplicative, choice("*", "/", "%")],
      ];

      return choice(
        ...table.map(([precedence, operator]) =>
          prec.left(
            precedence,
            seq(
              field("left", $.expression),
              field("operator", operator),
              field("right", $.expression),
            ),
          ),
        ),
      );
    },

    unary_expression: ($) =>
      prec.right(
        PREC.unary,
        seq(field("operator", choice("!", "-", "+")), $.expression),
      ),

    call_expression: ($) =>
      prec(
        PREC.call,
        seq(
          field("function", $._expression_member),
          field("arguments", $.argument_list),
        ),
      ),

    member_expression: ($) =>
      prec(
        PREC.member,
        seq(
          field("object", $._expression_member),
          ".",
          field("property", $.identifier),
        ),
      ),

    _expression_member: ($) =>
      choice(
        $.identifier,
        $.call_expression,
        $.member_expression,
        $.parenthesized_expression,
        $.self_expression,
      ),

    argument_list: ($) => seq("(", optional(commaSep($.expression)), ")"),

    parameter_list: ($) => seq("(", optional(commaSep($.parameter)), ")"),

    parameter: ($) =>
      seq(
        repeat($.annotation),
        field("name", $.identifier),
        optional(seq(":", field("type", $.type_expression))),
      ),

    parenthesized_expression: ($) => seq("(", $.expression, ")"),

    array_literal: ($) =>
      seq(
        "[",
        optional($._statement_terminator),
        optional(
          seq(
            $.expression,
            repeat(
              seq(
                optional($._statement_terminator),
                ",",
                optional($._statement_terminator),
                $.expression,
              ),
            ),
            optional($._statement_terminator),
            optional(","),
          ),
        ),
        optional($._statement_terminator),
        "]",
      ),

    object_literal: ($) =>
      prec.dynamic(
        -1,
        seq(
          "{",
          optional($._statement_terminator),
          optional(
            seq(
              $.object_field,
              repeat(
                seq(
                  optional($._statement_terminator),
                  ",",
                  optional($._statement_terminator),
                  $.object_field,
                ),
              ),
              optional($._statement_terminator),
              optional(","),
            ),
          ),
          optional($._statement_terminator),
          "}",
        ),
      ),

    object_field: ($) =>
      choice(
        seq(field("key", $.object_key), ":", field("value", $.expression)),
        field("key", $.identifier),
      ),

    object_key: ($) => choice($.identifier, $.string),

    prompt_identifier: (_) => token(/[A-Za-z_][A-Za-z0-9_]*/),

    boolean: (_) => choice("true", "false"),

    number: (_) => token(/[0-9]+(\.[0-9]+)?/),

    string: (_) => token(seq('"', repeat(choice(/[^"\\]+/, /\\./)), '"')),

    identifier: (_) => /[A-Za-z_][A-Za-z0-9_]*/,

    comment: (_) => token(seq("#", /[^\n]*/)),

    self_expression: (_) => "self",

    type_expression: ($) =>
      choice(
        $.identifier,
        $.member_expression,
        $.generic_type,
        $.function_type,
      ),

    generic_type: ($) =>
      seq(field("base", $.identifier), "<", commaSep1($.type_expression), ">"),

    function_type: ($) =>
      seq("fun", $.parameter_list, optional(seq("->", $.type_expression))),

    destructuring_pattern: ($) =>
      seq("{", commaSep1($.identifier), optional(","), "}"),

    _annotated_statement: ($) => seq(repeat($.annotation), $.statement),

    _statement_separator: ($) =>
      seq(
        choice(";", $._statement_terminator),
        repeat(choice(";", $._statement_terminator)),
      ),
  },
});

function commaSep(rule) {
  return seq(rule, repeat(seq(",", rule)));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
