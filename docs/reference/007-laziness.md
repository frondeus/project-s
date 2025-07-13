S-lang by default is an eager language.

However, sometimes you may want to postpone the evaluation of an expression (for example to create recursive type which we will cover later).

This can be done explicitly by using `thunk` special form:

```s
(let :x (thunk () 1))
```

```eval
val x : 1 = 1.0
- : () = []
```

Thunks are evaluated first time they are called. From the type system perspective thunks are invisible - they are represended by the inner type of the value that would be returned.

Note, how `eval` codeblock evaluated it as it wasn't there.
It's because in REPL printing is eager evaluating all thunks when possible.

We can enforce laziness of REPL in the markdown by using `lazy` attribute:

```s
(let :x (thunk () 1))
```

```eval lazy
val x : 1 = "<Thunk: Thunk/ToEvaluate>"
- : () = []
```

Now we can see, that even though thunk was not evaluated yet, it already has a type `1`.


# Captured variables

You may wonder what the first `()` means - It is a list of captured variables.
It is provided explicitly, but in most cases you can leave it empty.
If possible, **compiler will automatically capture all necessary variables** and populate this list.

Example:

```s
(let :a 1)
(let :x (thunk (a) a))
```

```eval lazy
val a : 1 = 1.0
val x : 1 = "<Thunk: Thunk/ToEvaluate>"
- : () = []
```

Here, thunk captures variable `a` - note we are not using `:a` keyword, but a symbol, because we are **refering** to existing `:a` binding.

And as mentioned earlier, we can leave that list empty:

```s
(let :a 1)
(let :x (thunk () a))
```

```eval lazy
val a : 1 = 1.0
val x : 1 = "<Thunk: Thunk/ToEvaluate>"
- : () = []
```

If we look at processed SExpressions, we will see, that compiler automatically captured `a`.

```processed
(top-level
  (let
    :a
    1)
  (let
    :x
    (thunk
      (a)
      a)))
```
