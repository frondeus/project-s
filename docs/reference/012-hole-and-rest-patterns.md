Well, I promised and I want to deliver my promise.
In this chapter we will learn about hole and rest patterns.

Let's start with hole patterns.

Sometimes you may want to destruct a tuple but ignore one of the items.

For example:

```s
(let :t (tuple 1 2 3))
```

Maybe you don't care about the second one.

You could in theory use two indexing operations:

```s
(let :t (tuple 1 2 3))
(let :first (index t 0))
(let :third (index t 2))
```

But it would be nicer to use a destructing pattern:

```s
(let (:first :second :third) (tuple 1 2 3))
```

In here we end up with unnecessary `:second` binding.

```eval
val first : 1 = 1.0
val second : 2 = 2.0
val third : 3 = 3.0
- : () = []
```

To avoid this, we can use "hole" pattern `_`:

```s
(let (:first _ :third) (tuple 1 2 3))
```

```eval
val first : 1 = 1.0
val third : 3 = 3.0
- : () = []
```

# Rest patterns

Okay, as promised lets quote me one more time:

The most basic requirement for a tuple is:
* Must allow random access to the element by the index.
* Must allow having heterogeneous elements.
* May (but not must) have a fixed size.

It's finally time to talk about the last point.

What if we want to destruct a tuple and access only first element?

Sure, we can use newly-introduced holes:

```s
(let (:first _ _) (tuple 1 2 3))
```

```eval
val first : 1 = 1.0
- : () = []
```

but what if we dont want to know the size of the tuple? What if, we want to just assert that it has **at least** one element?

To make it happen, S-lang provides a rest pattern:

```s
(let (:first .._) (tuple 1 2 3))
```

```eval
val first : 1 = 1.0
- : () = []
```

# Accessing rest

You may wonder, why we are using `.._` and not `..`?

The answer is - because in fact we are using two patterns: `..` and `_` on top of each other.
That allows us to replace `_` with named single parameter and get an access to the rest of the tuple:

```s
(let (:first ..:rest) (tuple 1 2 3))
```

```eval
val first : 1 = 1.0
val rest : [2 ∨ 3] = [
  2.0,
  3.0
]
- : () = []
```

You may wonder what the hekc is `[2 v 3]` syntax?. Well it is a **list**. But that, is a topic for another chapter.
