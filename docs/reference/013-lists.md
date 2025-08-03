In the last chapter we introduced the concept of lists by using `..:rest` pattern.

Rest pattern is returning a list of elements, so let's talk about it.

A list, like a tuple is a linear collection, that can be accessed by index.
However, unlike tuples, lists enforce the same type for all elements.

To create a list, we can use `list` function (surprising, isn't it?)

```s
(let :l (list 1 2 3))
```

```eval
val l : [1 ∨ 2 ∨ 3] = [
  1.0,
  2.0,
  3.0
]
- : () = []
```

# Syntax sugar

Since S-lang is a LISP, and lists are quite common, we can also use square brackets to define a list:

```s
(let :l [1 2 3])
```

```eval
val l : [1 ∨ 2 ∨ 3] = [
  1.0,
  2.0,
  3.0
]
- : () = []
```

# Ascription

We can always specify the type of a list:

```s
(let :l (: [:number] [1 2 3]))
```

```eval
val l : [number] = [
  1.0,
  2.0,
  3.0
]
- : () = []
```

# Covariance

Lists are covariant, which means that we can assign a list of a subtype to a list of a supertype.

```s
(let :f (:
    (fn ([:number]) _)
    (fn (:l) l)
))

(let :numbers (: [:number] [1 2 3]))
(let :ones (: [1] [1 1 1]))

(let :a (f numbers))
(let :b (f ones))

```

```eval
val f : ([number]) → [number] = "<Function: LispFn>"
val numbers : [number] = [
  1.0,
  2.0,
  3.0
]
val ones : [1] = [
  1.0,
  1.0,
  1.0
]
val a : [number] = [
  1.0,
  2.0,
  3.0
]
val b : [number] = [
  1.0,
  1.0,
  1.0
]
- : () = []
```

As you can see, even though `ones` have a type of `[1]`, we can safely pass them to `f`, and the result, b, has now more general type `[number]` but still contains only ones.

# Indexing a list

Of course we can also index the list by the number:

```s
(let :l (: [:number] [1 2 3]))
(l 0)
```

```eval
val l : [number] = [
  1.0,
  2.0,
  3.0
]
- : number = 1.0
```

# Comparing lists to tuples
If we once again quote our tuple requirements, and compare them to lists:

> The most basic requirement for a tuple is:
> * Must allow random access to the element by the index.
> * Must allow having heterogeneous elements.
> * May (but not must) have a fixed size.

We will see that lists:
* Still allow random access by the index
* Enforce homogeneous element
* Are indefinite in size

The last point means, that indexing a list will not trigger a type error if the index is out of bounds.

> [!TODO]
> If we ever introduce `None | Some()` it would be good to say that the indexing operation returns an option of the type.


# Destructing lists

Just like tuples, we can destruct lists into their elements, in fact, we can use precisely the same pattern.
However, since we don't know the size of the list, we **must** use a rest pattern

```s
(let (:first .._) (: [:number] [1 2 3]))
```

```eval
val first : number = 1.0
- : () = []
```

It of course means we can access the rest of the list that way:

```s
(let (:first ..:rest) (: [:number] [1 2 3]))
```

```eval
val first : number = 1.0
val rest : [number] = [
  2.0,
  3.0
]
- : () = []
```

> [!TODO]
> This is not very safe. What if we provide a list that is empty?
> In other functional languages (or Rust) that kind of binding would be forbidden.
> In Rust particularly, you would have to use either:
> * match
> * if let
> * let [first, rest @ ..] = list else { };

In other words binding would be introduced **only** if the list matches it in the runtime, but we would have to provide an alternative branch.

```s
(let (:first .._) [])
first
```

```eval
val first : 'a
- : 'a = "<Error: Runtime error: Undefined variable: first>"
```
