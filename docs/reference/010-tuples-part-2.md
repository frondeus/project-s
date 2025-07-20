There is one topic i did not cover yet.

When I defined tuples, I said:

> The most basic requirement for a tuple is:
> * Must allow random access to the element by the index.
> * Must allow having heterogeneous elements.
> * May (but not must) have a fixed size.

Sure, second point is covered, and third point I promised to cover in the future.
But what about the first point?

Besides destructuring (which for now enforces us to define precise size of a tuple, we can't destruct only first two elements of triple), we haven't got support for accessing random element of a tuple.


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


# Application

What if we define a function that accesses the first element of a tuple?
What would be the type of such function?

How S-lang knows that `(t 1)` is "accessing" tuple, and not calling a function with single numeral parameter?

The answer is - S-lang has no idea.
All it can infer is that "`t` is something that must be able to be called with a tuple of `(1)`".
And that is it!

So if we look at such a function:

```s
(let :f (fn (:t)
  (t 0)
))

(let :t (tuple 1 2 3))

(f t)
```

```eval
val f : forall ((0) -?-> 'a) → 'a = "<Function: LispFn>"
val t : (1, 2, 3) = [
  1.0,
  2.0,
  3.0
]
- : 1 = 1.0
```

You can see that `f` has a type `forall ((0) -?-> 'a) → 'a`. which can be read as:
"For every `'a` f takes a tuple of with something ''applicative'' that can take a tuple of `(0)` and return `'a`."
In the type system currently we encode that "applicative" type as `{left} -?-> {right}`.

Another way of seeing it, is as if we had a trait (using Rust notation):

```rust
trait Applicative<Args> {
    type Output;

    fn apply(self, args: Args) -> Self::Output;
}
```

That trait is implemented for tuples (well. imagine if it was implemented because in reality it cannot be expressed in Rust):

```rust
impl Applicative<(usize)> for (T1,T2,...) {
    type Output = <depending on arg return T1, T2 etc>;

    fn apply(self, args: ()) -> Self::Output {
        self.0
    }
}
```

Yet another (and more correct tbh) way of looking it is as if we had a TYPE.

> [!NOTE]
> This is just an example of non-existing semantics and syntax.

```example
type Applicative<Args, Ret>;
```

and then tuple is a **subtype** of Applicative:

```example
type Tuple<(T1, T2)>
  where Self :< Applicative<(0,), T1>
  and Self :< Applicative<(1,), T2>;
```

And **Function** is also a subtype of Applicative:

```example
type Function<T1, T2>
  where Self :< Applicative<T1, T2>;
```

In other words in our example, since `f` is polymorphic, we can use it with both tuples
and functions because both are a subtype of this Applicative type.

```s
(let :f (fn (:t)
  (t 0)
))

(let :t (tuple 1 2 3))
(let :id (fn (:x) x))

(tuple
  (f t)
  (f id)
)
```

```eval
val f : forall ((0) -?-> 'a) → 'a = "<Function: LispFn>"
val t : (1, 2, 3) = [
  1.0,
  2.0,
  3.0
]
val id : forall ('a) → 'a = "<Function: LispFn>"
- : (1, 0) = [
  1.0,
  0.0
]
```
