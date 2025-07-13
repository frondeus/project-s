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
Error: Unreadable pattern: Expected keyword or list, found: Symbol("_")

```
