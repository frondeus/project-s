So far we were using `..` only in negative polarity.

(Wait, negative polarity? What's that?)

Basically we were using `..` only to describe "usage of the type" - in patterns, whether it was in the function parameter or let binding:

```s
(let :first (fn (:a .._) a))

(first 1 2 3)
```

```eval
val first : forall ('a, ..'b) → 'a = "<Function: LispFn>"
- : 1 = 1.0
```

But, could we perhaps use it in the positive polarity, to define a new value?

Kinda.

S-lang shamelessly stole the idea from Janet.
I love Janet, although she's not-a-girlfriend.

Janet is cool.

Anyway. The idea was to use `..` for "splices":
Splices allow us to take a list or a tuple and insert its elements into another existing list or tuple.

Let's look at the example below:

Let's say we want to add numbers together.
We have a function `+` that takes a list of numbers and returns their sum:

```s
+
```

```eval
- : [number] → number = "<Function: RustFn>"
```

Note, that this function does not take a tuple with list as first item. No. It takes a list.

Meaning the use is like this:

```s
(+ 1 2 3)
```

```eval
- : number = 6.0
```

But what if you actually want to pass a list?

```s
(let :l [1 2 3])
(+ l)
```

That is not allowed, since we can't pass a list directly.

```eval
Error: Type mismatch
    ╭─[ <builtin>:10:24 ]
    │
  1 │ "+": [number] -> number
    │       ───┬──  
    │          ╰──── Expected number
    │ 
 10 │ "list": forall ['a] -> ['a]
    │                        ──┬─  
    │                          ╰─── But found list
────╯

```

Yeah, not what we wanted.

So the `splice` operator known as `..` is coming here for to a rescue:

```s
(let :l [1 2 3])
(+ ..l)
```

```eval
Error: Undefined variable: splice
   ╭─[ <input>:2:4 ]
   │
 2 │ (+ ..l)
   │    ─┬─  
   │     ╰─── Used here
───╯

```
