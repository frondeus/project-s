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

  rules: {
    source_file: $ => repeat($._sexp),

    _sexp: $ => choice(
      $.quote,
      $.float,
      $.integer,
      $.string,
      $.symbol,
      $.list
    ),


    list: $ => seq(
      '(',
      repeat($._sexp),
      ')'
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
    string_inner: $ => /[^"]*/,
    symbol: $ =>  token(prec(1, /[^\s()'"]+/)),
    float: $ =>   token(prec(2, /[+-]?(?:[0-9]+\.[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?/)),
    integer: $ => token(prec(2, /[+-]?[0-9]+/)),
  }
});
