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

Ok, now reader shortcut.

```
`(1 2 ,(+ 1 2))
```

```json
"(1 2 3)"
```

Cool. Now can we use it in our struct definition?

```
(struct `(
  :name "John Smith"
  :age ,(+ 20 3)
))
```

```json
{
  "age": 23.0,
  "name": "John Smith"
}
```

NICE.

Okay. What's next?
It would be nice to be able to add two
objects together.
However, I think first i need to address lack of variables.

```
(let x 2 (+ x x))
```

```json
4.0
```

So now just using symbol should return error
```
x
```

```json
"<Error: Undefined variable: x>"
```

Okay, just like jsonnet i want to be able to create new variables inside of structs.

```
`(:key (+ 1 2))
```

```json
"(:key (+ 1 2))"
```


Apparently we have bug in quasiquoting.
Which isn't surprising.
The problem is:

```
(struct `(
  :key '(+ 1 2)
))
```

```json
{
  "key": "(+ 1 2)"
}
```


The quasiquote returns SExpression that is generation further
than the parent.

How? Currently quasiquoting is creating new AST.
That AST contains (+ 1 2) expression.
But later, We keep it as a reference and when we try to print that expression,
the AST is lost. So we have a dangling pointer of sorts.

Ok fixed.
The only downside is - it's going to leak memory over time.
But we need GC anyway...

Ok, going back to let expressions

```
(struct `(
  (let x 5)
  :key (+ 1 x)
  :another '(+ 1 x)
))
```

```json
{
  "another": "(+ 1 x)",
  "key": 6.0
}
```

Oooh, I get why this is not working.

The problem is, that unquoting happens BEFORE we start digesting struct.
Which fucking make sense. That is intended behaviour.
We do quasiquote and only AFTER that we pass that to the struct function
in order to generate struct from SExp.

The only thing i can do is to reverse the inner quoting.
Bingo.

Ok, what about "self"?

There are two options i see.
Either I choose lazy evaluation, or I need to somehow sort the field initialization by the DAG. Assuming there is a DAG.

For example if we have:

```example
(struct `(
  :key (+ 1 self.another)
  :another (+ 1 1)
))
```

then we need to reorder the struct into

```example
(struct `(
  :another (+1 1)
  :key (+ 1 self.another)
))
```
because then `:another` key exists in env.

Another point is - in order to use `self` we need struct accessing
first.

As in accessing field of the struct

The simplest way is to have it like this:

```
( 
  (struct '(:key 1 :another 2))
  :another
)
```

```json
2.0
```

In other words, structure is used as if it was a function and first
parameter tells you the key accessed

It works even better if struct is named

```
(let foo (struct '(:key 1 :another 2))
  (foo :another)
)
```

```json
2.0
```

Cool. So, `self`...

Let's say that for now keys HAVE TO
be ordered explicitly

```
(struct `(
  :another (+ 1 1)
  :key (+ 1 (self :another))
))
```

```json
{
  "another": 2.0,
  "key": 3.0
}
```

Okay, but how do i access object? It's not in the env. And it cannot be yet
in some Arc<_> because it is still being mutated.

Ok, now next one has to fail, right?

```
(struct `(
  :key (+ 1 (self :another))
  :another (+ 1 1)
))
```

```json
{
  "another": 2.0,
  "key": "<Error: Undefined key: another>"
}
```

Yep.

Okay, now the only thing missing is "root".

```
(struct `(
  :another 4
  :key (struct '(
    :a 1
    :b (+ 1 (root :another))
  ))
))
```

```json
{
  "another": 4.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  }
}
```

Damn, that's cool!

Does it work with accessing self?

```
(struct `(
  :another 4
  :key (struct '(
    :a 1
    :b (+ 1 ((root :key) :a))
  ))
))
```

```json
{
  "another": 4.0,
  "key": {
    "a": 1.0,
    "b": "<Error: Unknown value: Error(\"Undefined key: key\")>"
  }
}
```

No. It doesnt because key was not created yet. Makes sense.

Ok, now i want to have another reader shortcut

```
{
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}
```

```json
{
  "another": 4.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  }
}
```

# 25.05

Okay. What else am I missing?

I guess one reason I started this project is to be able to add two structs
together like in the JSONnet.

So... Let's maybe focus on it.

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

{
  :another 9
})
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  }
}
```

Oooh... Okay. I see.
It is returning `b` = `5.0` because overriding `another` doesn't really
change `b`. Because `b` was already calculated in the left side.
And since `b` is not lazy... 

So I expected a 10.0 but that is impossible with the current lang.
Damn. That's why jsonnet is a lazy language, right?

Okay let's not focus right now on it. Maybe it won't be such an issue.
I want to add `super`!

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

{
  :another 9
  :self (+ 1 (self :another))
  :super (+ 1 (super :another))
})
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  },
  "self": 10.0,
  "super": 5.0
}
```

For now we are cloning supers over and over. That should
change but I don't have time or energy to do it now.
Maybe what we need is some kind of reference value in the future.

Okay, what if I override nested struct?

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

{
  :another 9
  :key 10
})
```

```json
{
  "another": 9.0,
  "key": 10.0
}
```

Makes sense.

Okay. Next thing is - If i want to have a partial adding of the struct,
I need to have conditionals on the keys.

That means, I need proper booleans.


```
true
```

```json
true
```

```
false
```

```json
false
```

Ok now the conditionals inside of objects

```
{
  (if true '(:key 42))
  (if false '(:false 13))
  (if false '(:true 10) '(:else 12))
}
```

```json
{
  "else": 12.0,
  "key": 42.0
}
```