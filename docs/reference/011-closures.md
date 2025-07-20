When talking about function type we cannot ommit it's younger brother - the closure.

Currently closures are sharing the same type as functions (we kinda ignore captured variables in the type system).

In fact, in most cases you can also ignore the existence of closures and use functions everywhere. S-lang compiler will detect automatically when a function has a free variable (as in variable that is not defined in the function itself), and add it to the list of captured expressions.

Let's define our first closure:

```s
(let :x 1)
(let :f (fn () x))
```

```eval
val x : 1 = 1.0
val f : forall () → 1 = "<Function: LispFn>"
- : () = []
```

See? Not scary at all!. `f` has captured `x`.

But what if `x` is defined in inner scope, is that working as well?

```s
(let :outer (fn () (do
    (let :x 2)
    (let :inner (fn () x))
    inner
)))

(let :f (outer))

(f)
```

```eval
val outer : forall () → () → 2 = "<Function: LispFn>"
val f : () → 2 = "<Function: LispFn>"
- : 2 = 2.0
```

Even though `:inner` captures `:x`, and `:x` is no longer in a scope when we call `f`, it still has an access to it.

It is because currently capturing variables == Cloning them.

> [!TODO]
> Make sure we can capture variables without introducing such a big memory overhead.

Note, also that `f` is not polymorphic - because in order to create it we had to call `outer`. It is to prevent nasty kind of bugs in the type system when mutable references are in use.


# Capturing

So, as I mentioned, S-lang automatically detects which variable has to be captured.

If we look at the inner S-expression representation (like we did with thunks) you will see that `fn` is replaced with `cl` that takes another argument - a list of captured variables, just like `thunk` does!.

```processed
(top-level
  (let
    :outer
    (fn
      ()
      (do
        (let
          :x
          2)
        (let
          :inner
          (cl
            ()
            (x)
            x))
        inner)))
  (let
    :f
    (outer))
  (f))
```
