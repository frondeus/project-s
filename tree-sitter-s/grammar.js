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
  
  word: $ => $.symbol,
  rules: {
    source_file: $ => repeat($._sexp),

    _sexp: $ => choice(
      $.quote,
      $.quasiquote,
      $.unquote,
      $.struct,
      $.array,
      $.splice,

      $.list,
      $.float,
      $.integer,
      $.string,
      $.boolean,
      $.keyword,
      $.symbol,
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

    splice: $ => seq(
      "..",
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
    float: $ =>   /[+-]?(?:[0-9]+\.[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?/,
    integer: $ => /[+-]?[0-9]+/,
    keyword: $ => seq(":", /[^\s)}\]]+/),
    symbol: $ =>  token(prec(-10, /[^\s)}\]]+/)),
  }
});
