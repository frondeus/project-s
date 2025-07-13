There is one topic i did not cover yet.

When I defined tuples, I said:

> The most basic requirement for a tuple is:
> * Must allow random access to the element by the index.
> * Must allow having heterogeneous elements.
> * May (but not must) have a fixed size.

Sure, second point is covered, and third point I promised to cover in the future.
But what about the first point?

Besides destructuring (which for now enforces us to define precise size of a tuple, we can't destruct only first two elements of triple), we haven't got support for accessing random element of a tuple.

> [!TODO]
> We should add support for destructuring with rest pattern.

S-lang takes an inspiration from other LISP languages and adds support for accessing by index using the same form as function call.

It means, if the first argument of the SEXp list `(t 1)` is a tuple, we "emulate" as it was a function that takes index as a parameter.

```s
(let :t (tuple 1 2 3))
(t 1)
```

```eval
val t : (1, 2, 3) = [
  1.0,
  2.0,
  3.0
]
- : 2 = 2.0
```

Note, tuples are zero-indexed, so by asking for `(t 1)` we access the second element of the tuple.
