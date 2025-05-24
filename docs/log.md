# What I want

I want to create simple LISP-like language.
It should have a good LSP support and static typing (HM-inference).

It should be created incrementally, starting with just numbers.

I want to expand it horizontally not vertically, so in the beginning the language would support only numbers (evaluating constants) but already have working LSP, REPL, evaluation and acceptance tests.


# Long term goal

I want to have something similar in functionality to Jsonnet (https://jsonnet.org) especially i like adding two structs together. That is brilliant and exactly what i want from my lang.


# Log 14.05
Added tree-sitter-s. It's parsing CST. Added `parser.rs` - its converting CST into AST.

# Log 19.05
Added integration tests in markdown.
Lets check it:

```
(1 2 3)
```

```cst
List([Number(1.0), Number(2.0), Number(3.0)])
```

Cool!.

XTask was also added. Now we can start writing the language itself!

Okay, so what I want?
I want to at least have

```
(struct (quote (
    :name "Name"
    :surname "Surname"
)))
```

and have it compiled to 

```json
{
  "name": "Name",
  "surname": "Surname"
}
```

Let's write the execution of such

# Log 20.05

Okay. That works.
But clearly i will not trust Cursor for long. Let's do it properly, manually and with the right approach.

Let's start with a number:

```
5
```

```json
5.0
```

# Log 21.05

Okay, now a string. I will definitely need a string. 
For now simple string will be enough. No escaping, nothing that fancy

```
"5"
```

```json
"5"
```

Cool. Before I go next. I think its time to extend the language vertically.
First - static typing

But before we do that, maybe its time to express our tree as a flat array 
Now that we have it, we can apply some basic typechecking

```
5
```

```type
Number
```

```
"41"
```

```type
String
```

Cool. Now the next thing should be type ascription in the expression

```
(is-type 5 Number)
```

```json
true
```

```
(is-type 5 String)
```

```json
false
```

```
(is-type "5" Number)
```

```json
false
```

```
(is-type "5" String)
```

```json
true
```

# 24.05

Cool. It's not finished but at least we started doing something.
If it's lisp it should allow quoting:

```
(quote 1)
```

Primitive atoms are evaluating to themselves

```json
1.0
```

```
(quote "42")
```

```json
"42"
```

Symbols are evaluating to themselves as well

```
(quote quote)
```

```json
"quote"
```

Any advanced sexp is just kept as is.

```
(quote (1 2 3))
```

Printing to JSON is wrapping sexp in Strings
```json
"(1 2 3)"
```

Now we can use this quote to actually properly construct struct

```
(struct (quote (
    :name "Name"
    :surname "Surname"
)))
```

and have it compiled to 

```json
{
  "name": "Name",
  "surname": "Surname"
}
```

For now we treat as if struct was a function that took SExpression.
Later when we introduce macros, we might want to change that.

For now, we use full `quote`. But it makes sense to expand the grammar to introduce `'`.

```
(struct '(
    :name "Name"
    :surname "Surname"
))
```

and have it compiled to 

```json
{
  "name": "Name",
  "surname": "Surname"
}
```

Ok. Cool, this is nice and shit, but this language is useless if we cannot do any operation.
Let's for now add hardcoded add operator

```
(+ 1 2)
```

```json
3.0
```

Now we need a way to calculate that op as a field of struct.
Which means quasiquote and unquote operators.

This is using quote:
```
'(1 2 (+ 1 2))
```

```json
"(1 2 (+ 1 2))"
```

But this is using quasiquote

```
(quasiquote (1 2 (+ 1 2)))
```

Still no difference:

```json
"(1 2 (+ 1 2))"
```

Until we use unquote

```
(quasiquote (1 2 (unquote (+ 1 2))))
```

```json
"(1 2 3)"
```


However in order to make it work, i need to first:
* Make sure we can create new AST on the fly.
* Make sure that when we are accessing SExp, we are asking the right AST tree.

The easiest solution is to keep reference to AST inside of SExp.
But... where?

Or maybe not. Lifetimes will eat me alive.
Maybe i just need to clone the whole quasiquoted tree.

Yep. For now cloning will work.