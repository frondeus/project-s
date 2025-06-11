/**
 * @file Project S
 * @author Wojciech Polak <project-s@frondeus.pl>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "s",

  // conflicts: $ => [
  //   [
  //     [$.float, $.symbol],
  //     // [$.integer, $.symbol]
  //   ]
  // ],
extras: $ => [
  /\s+/,
  $.comment,
],
  
  rules: {
    source_file: $ => repeat($._sexp),

    _sexp: $ => choice(
      $.quote,
      $.quasiquote,
      $.unquote,
      $.struct,
      $.array,

      $.float,
      $.integer,
      $.string,
      $.boolean,
      $.keyword,
      $.symbol,
      $.list
    ),

    comment: $ => seq(
      "#", /[^\n]*/
    ),

    list: $ => seq(
      '(',
      repeat($._sexp),
      ')'
    ),

    struct: $ => seq(
      '{',
      repeat($._sexp),
      '}'
    ),

    array: $ => seq(
      '[',
      repeat($._sexp),
      ']'
    ),

    quasiquote: $ => seq(
      "`",
      field("inner", $._sexp)
    ),

    unquote: $ => seq(
      ",",
      field("inner", $._sexp)
    ),

    quote: $ => seq(
      "'",
      field("inner", $._sexp)
    ),

    string: $ => seq(
      '"',
      field("inner", $.string_inner),
      '"'
    ),
    boolean: $ => token(prec(2, choice(
      "true",
      "false"
    ))),
    string_inner: $ => /[^"]*/,
    keyword: $ => token(prec(2, /:[^\s()'"`,{}\[\]#]+/)),
    symbol: $ =>  token(prec(1, /[^\s()'"`,{}:\[\]#]+/)),
    float: $ =>   token(prec(2, /[+-]?(?:[0-9]+\.[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?/)),
    integer: $ => token(prec(2, /[+-]?[0-9]+/)),
  }
});
