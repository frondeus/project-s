So far we were only operating on primitive literals.
However that alone does not make a language.

In order to talk about last atom expression (symbol), first we need to define a variable and variable definitions.

S-Lang is lisp based syntax.

In order to define a variable, we use `let` special form.
Note, that the name itself is a keyword!.

```s
(let :x 10)
```

```eval
val x : 10 = 10.0
- : () = []
```

First of all, `:x` is a pattern here, used to name the variable but also can be used to destructure the value.

Secondly, the `let` is an expression, so it has to return a value.
At the same time to make it more consistent with other languages we return `()` type/expression called Unit.

And finally, in contrast to ml-family of languages, `let` does not create a new scope (In ml we would use `let x = 10 in ...;`). Instead, it adds a binding to existing scope.

# Scopes

By default every `S` program contains two scopes: prelude & top-level.
Both are added automatically and are usually invisible to the user.

# Symbols

In order to use a variable, we can use symbol expression - `x` without `:` prefix:

```s
(let :x 10)
x
```

```eval
val x : 10 = 10.0
- : 10 = 10.0
```
