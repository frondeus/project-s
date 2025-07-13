So far we were avoiding the topic of functions and function calls.
Well. Let's say that in this chapter we will continue that trend ;-)

But more seriously, currently we can't talk about functions without tuples.

A tuple is a linear collection of values, where every value might have different type.

The most basic requirement for a tuple is:
* Must allow random access to the element by the index.
* Must allow having heterogeneous elements.
* May (but not must) have a fixed size.

The last point is controversial one.
In other languages usually tuples MUST be fixed size.
However, since tuples are so essential part of function calls (i mean, arguments provided to the function are just a tuple), and I want to support variable number of arguments, ==> tuples may have a variable size.

But, that is like a special case of normal boring tuple.
So let's start with boring :)

In order to define a tuple, we introduce a new special form.

```s
(let :x (tuple 1 2 "3"))
```

```eval
val x : (1, 2, "3") = [
  1.0,
  2.0,
  "3"
]
- : () = []
```

## Type ascription

```s
(let :x (: (tuple :number 2 "3") (tuple 1 2 "3"))
```

```eval
val x : (number, 2, "3") = [
  1.0,
  2.0,
  "3"
]
- : () = []
```

## Destructing

Since we introduced first complex type (a type that is not a primitive or literal), we can now introduce other pattern - destructing a tuple.

```s
(let (:a :b) (tuple 1 2))
```

```eval
val a : 1 = 1.0
val b : 2 = 2.0
- : () = []
```

Destructing pattern enforces the shape of a tuple, but not type of elements.

The same pattern can be used to destructure argument of a function...
But that is a topic for separate chapter.
