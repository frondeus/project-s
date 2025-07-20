So far all constructs introduced in S-lang were **immutable**.

However, This is quite restricting.
S-lang, again, takes an inspiration from ML-family languages and introduces references.

Note, those references are different than what you know from `Rust` or `C++`,
these are more similar to `RefCell<T>` from Rust than `&mut T`.

In order to create a mutable reference you can use the `ref` special form:

```s
(let :x (ref 10))
```

```eval
val x : refmut 10 = 10.0
- : () = []
```

Now, there are two operations one can do with a reference:
* Read it with `get`
* Write it with `set`

# Reading

In order to read a reference you can use `get` function:

```s
(let :x (ref 10))
(get x)
```

```eval
val x : refmut 10 = 10.0
- : 10 = 10.0
```

# Writing

In order to write a reference you can use `set` function.
Note we set the value of a reference to `:number`.

```s
(let :x (ref (: :number 10)))
(set x 20)
(get x)
```

```eval
val x : refmut number = 20.0
- : number = 20.0
```

What if we skip type ascription?

```s
(let :x (ref 10))
(set x 20)
(get x)
```

We get an error, because we made a reference that only accepts "10", and not "20".

```eval
Error: Type mismatch
   ╭─[ <input>:2:8 ]
   │
 1 │ (let :x (ref 10))
   │              ─┬  
   │               ╰── Expected 10
 2 │ (set x 20)
   │        ─┬  
   │         ╰── But found 20
───╯

```


# Ascribing


By default `ref` special form creates a readable and mutable reference.

However, we can restrict the type to just readable reference (when for example passing it to a function) to prevent mutation in part of our program:

```s
(let :x (ref 10))
(let :only-read (:
    (fn ((ref :number)) :number)
    (fn (:x) (get x))
))

(only-read x)

```

```eval
val x : refmut 10 = 10.0
val only-read : (ref number) → number = "<Function: LispFn>"
- : number = 10.0
```

To prove that it works, lets try to set the x inside of `only-read`:

```s
(let :x (ref 10))
(let :only-read (:
    (fn ((ref :number)) :number)
    (fn (:x) (do
        (set x 20)
        (get x)
    ))
))

(only-read x)

```

```eval
Error: Reference is not writable
   ╭─[ <input>:3:10 ]
   │
 3 │     (fn ((ref :number)) :number)
   │          ──────┬──────  
   │                ╰──────── This reference is not writable
   │ 
 5 │         (set x 20)
   │         ─────┬────  
   │              ╰────── Expected here
───╯

```

# Type signature

You may have noticed that in the type signature `refmut` is used instead of `ref`.
This is because references are split into two parts - siblings.
Readable type and writable type.

In most cases, we write `refmut T` as a shortcut for `ref T mut T`.
If we create mutable only reference, we would write `mut T`.

We can set it but we can never retrieve value.
```s
(let :r (: (mut 10) (ref 10))
```

```eval
val r : mut 10 = 10.0
- : () = []
```


If we create immutable only reference, we would write `ref T` (just like we did in the last example with `ref number`!).

And technically we could also have `ref T mut U` however that is currently not so useful.

```s
(let :r (:
    (ref :number mut 10)
    (ref 10)
))

```

This reference returns a number, but can be only set to 10!

```eval
val r : ref number mut 10 = 10.0
- : () = []
```

Let's try such a weird construct:


```s
(let :r (:
    (ref :number mut 10)
    (ref 10)
))

(let :retrieved (get r))
(set r 10)
```

```eval
val r : ref number mut 10 = 10.0
val retrieved : number = 10.0
- : () = 10.0
```

Okay, so retrieved is properly typed as a `number`. But does it restrict us from setting the reference to 20?


```s
(let :r (:
    (ref :number mut 10)
    (ref 10)
))

(let :retrieved (get r))
(set r 20)
```

```eval
Error: Type mismatch
   ╭─[ <input>:7:8 ]
   │
 2 │     (ref :number mut 10)
   │                      ─┬  
   │                       ╰── Expected 10
   │ 
 7 │ (set r 20)
   │        ─┬  
   │         ╰── But found 20
───╯

```

Yes!

Why is that useful tho?

> [!TODO]
> Revisit that parapgraph after adding enums:

Well, lets imagine we are using Option type `Option<T>`.
That weird asymetric reference allows us to retrieve `Some(t)` value but we can only mutate it to make it `None`.
In other words, we guarantee that the option wont change it some other `Some(u)` value. Quite handy, huh?
