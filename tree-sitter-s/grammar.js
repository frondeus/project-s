/**
 * @file Project S
 * @author Wojciech Polak <project-s@frondeus.pl>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "s",

  rules: {
    source_file: $ => repeat($._sexp),

    _sexp: $ => choice(
      $.symbol,
      $.list
    ),

    symbol: $ => /[^\s()]+/,

    list: $ => seq(
      '(',
      repeat($._sexp),
      ')'
    ),
  }
});
