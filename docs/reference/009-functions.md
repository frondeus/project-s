Fine. Let's talk about functions.

First of all, **technically** a function only takes a single argument and returns a single value.

However, **practically** one can use a function only with tuple arguments
in order to do anything useful.

It is because of function application.

Okay, let's start with that, we can call abound defining function later.

Fortunately, S-Lang provides a basic prelude that includes useful functions.
One of them is `-` that subtracts two numbers.

```s
-
```

```eval
- : (number, number) → number = "<Function: RustFn>"
```

Since it is LISP-like language, in order to call such a function we are using `(- 2 1)` expression:

```s
(- 2 1)
```

```eval
- : number = 1.0
```

That leads to the most important part - currently there is no way to call a function and pass anything else than a tuple.

If that was ml-like language we would have it as:

```ocaml
- (2, 1)
```

So technically we could do

```ocaml
my-cool-fn 1
```
so pass a number directly to a function.

Bun in S-Lang? Nah. Always tuples!

So why having only one arg?
It's simpler to reason on a type inference level about a function that has only `{lhs} -> {rhs}` form.

Also, by keeping it that way, we can have orthogonal features where we use **destructing** (like introduced in previous chapter) to access function parameters.

And if we for example introduce variable number of parameters, it would work both in function params and in let expressions. Two birds with one stone!.


The biggest disadvantage of it, is that we loose famous currying - Currently it is not possible to pass only a part of a tuple to a function in order to get another function accepting the rest.

> [!TODO]
> Maybe that's something we could do with `(fn/curry f 1 2 3)` function call?

# Definitions

Okay, now that we handled calling, we can finally talk about defining!

To define a function, we use `fn` special form:

```s
(fn (:a :b) (- a b))
```

Here, we defined an anonymous function that takes two numbers and returns their sub. It's basically an alias of `-`.

```eval
- : (number, number) → number = "<Function: LispFn>"
```

So now we can call it:

```s
(let :f (fn (:a :b) (- a b)))

(f 2 1)
```

```eval
val f : forall (number, number) → number = "<Function: LispFn>"
- : number = 1.0
```

# Ascriptions

Of course just like with primitives and tuples we can explicitly type our functions
with type ascriptions:

```s
(let :f (:
    (fn (:number) :number)
    (fn (:a) a)
))
```

```eval
val f : (number) → number = "<Function: LispFn>"
- : () = []
```

## Type holes

Since we introduced complex types, we can also introduce a new type acription: `_`.
`_` allows you to say "i dont know". That allows us to only partially define the shape of a function and
leave the rest to the inferer. For example in the previous example we defined a function that took one number and returned a number. We can also re-define it by writing:


```s
(let :f (:
    (fn (:number) _)
    (fn (:a) a)
))
```

We let the compiler to infer that, returned type is the same as the input type, so the result is precisely the same:

```eval
val f : (number) → number = "<Function: LispFn>"
- : () = []
```

# Let polymorphism

You may notice that the type of `f` has `forall` prefix - it is a polymorphic function (although in that particular case it is not very useful, since all parameters are constrained to be numbers).

We can use it to define an identity function and call it with different parameters:

```s
(let :id (fn (:x) x))

(id 1)
(id "hello")
```

Here it makes more sense, since we define a function that takes ANY type `'a` and returns the same type `'a`.

```eval
val id : forall ('a) → 'a = "<Function: LispFn>"
- : "hello" = "hello"
```

## Value restriction

Like ocaml, S-lang restricts whenever binding is polymorphic.
For example it is forbidden to define a polymorphic function when function-call is used:

```s
(let :id (fn (:x) x)) # id is polymorphic
(let :mono (id (fn (:x) x))) # but mono is not. because right side of let expression has function call.

(mono 1)
```

```eval
val id : forall ('a) → 'a = "<Function: LispFn>"
val mono : ('a) → 1 = "<Function: LispFn>"
- : 1 = 1.0
```

> [!TODO]
> Technically `mono` should have a type `(1) -> 1`. It's a matter of coalescing infered type into readable type.

# Destructing

Technically, when we define a function we use a form `(fn <pattern> <body>)`.
It means that `(:x)` is a pattern, just like in `(let (:x) ...)`.

So far we learned two patterns, destructing tuples `(:x)` and single pattern `:x`.
The same **can** be applied to function patterns:

```s
(fn :x x)
```

```eval
- : 'a → 'a = "<Function: LispFn>"
```

Comparing it to `(fn (:x) x)`, `(fn :x x)` takes **any number of arguments** and returns all arguments (as a tuple), while `(fn (:x) x)` takes **one argument** and returns it.

So, lets try!

```s
(let :f (fn :x x))
(f 1 2 3)
```

```eval
val f : forall 'a → 'a = "<Function: LispFn>"
- : (1, 2, 3) = [
  1.0,
  2.0,
  3.0
]
```

So `f` is basically the same as `tuple` form. In fact, `tuple` form is not really a special form, but just another function defined in the prelude!

```s
tuple
```

```eval
- : 'a → 'a = "<Function: RustFn>"
```
