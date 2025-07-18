Okay, so far we have introduced primitive values and linear data structures - tuples and lists.

In this chapter I would like to talk about records.
In some languages these are known as structs, but in S-lang we take a legacy from ML languages.

A record, is a data structure that takes names fields. Basically it is like a HashMap or Rust struct.

To create a plain record one can use `obj/plain` (please don't mind `obj/` prefix, it should be `record/` but lets say that I was still in a phase of calling those objects and structs).


```s
(obj/plain)
```

```eval
- : {} = {}
```

A plain empty record is a bit boring, let's create another one with a field.
Each field (just like a variable binding) must start with `:` prefix:

```s
(obj/plain
   :key "value"
)
```

```eval
- : {key: "value"} = {
  "key": "value"
}
```

# Indirect key

A cool feature is to instead providing `:key` as a literal, to provide a variable instead.
As long as the variable points to statically known, const literal we are good to go:

```s
(let :mykey :key)

(obj/plain mykey "value")
```

```eval
val mykey : :key = ":key"
- : {key: "value"} = {
  "key": "value"
}
```

# String syntax sugar

Since records are popular, S-lang provides a syntax sugar with `{}` brackets.

```s
{
    :key "value"
}
```

```eval
- : {key: "value"} = {
  "key": "value"
}
```

# Accessing fields

Just like with tuples and lists, we can get the field value by using application.
You just need to pass the field name as a keyword:

```s
(let :o { :a 1 :b 2 :c 3 })

(o :b)
```

```eval
val o : {a: 1, b: 2, c: 3} = {
  "a": 1.0,
  "b": 2.0,
  "c": 3.0
}
- : 2 = 2.0
```

And just like with indirect object creation, the key may come from the variable, as long as it is a statically known, const literal:

```s
(let :o { :a 1 :b 2 :c 3 })
(let :key :b)

(o key)
```

```eval
val key : :b = ":b"
val o : {a: 1, b: 2, c: 3} = {
  "a": 1.0,
  "b": 2.0,
  "c": 3.0
}
- : 2 = 2.0
```

# Ascribtion

Of course we can use records in type ascription, for example to make it more general with `:number` type instead of integer literals:

```s
(let :o (:
   { :a :number :b :number }
   { :a 1 :b 2 }
))
```

```eval
val o : {a: number, b: number} = {
  "a": 1.0,
  "b": 2.0
}
- : () = []
```

# Record as function parameter

What if we want to pass record as a function parameter?
It is possible of course, as long as the parameter is a member of a tuple (otherwise there is no way to call that function!):

```s
(let :f (:
    (fn ({:a :number}) :number)
    (fn (:obj)
        (obj :a)
    )
))
```

```eval
val f : ({a: number}) → number = "<Function: LispFn>"
- : () = []
```

Now we can call such a function:


```s
(let :f (:
    (fn ({:a :number}) :number)
    (fn (:obj)
        (obj :a)
    )
))

(f { :a 1 })
```

```eval
val f : ({a: number}) → number = "<Function: LispFn>"
- : number = 1.0
```

But what if we want to pass a record that has more fields? Is that allowed?


```s
(let :f (:
    (fn ({:a :number}) :number)
    (fn (:obj)
        (obj :a)
    )
))

(f { :a 1 :b 2 })
```

```eval
val f : ({a: number}) → number = "<Function: LispFn>"
- : number = 1.0
```

As you can see, yes it is!
Even though, we provided a record with more fields than expected, the function call is still valid because the function only is interested in the existence of `:a`.

In a way you could say that `{ :a :number :b :number }` is a subtype of `{ :a :number }` because whenever `{ :a :number }` is needed, `{ :a :number :b :number }` can be also used as a substitute.

This also gives us an extra important property of S-lang - it is all about **structural typing** in the contrast to nominal typing

# Record destruction

Both in let expressions and function declarations we can also destruct records.

```s
(let { :a } { :a 1 })
```

```eval
val a : 1 = 1.0
- : () = []
```

However, sometimes one might want to rename the field

```s
(let { :a b } {:a 1 })
```

```eval
val b : 1 = 1.0
- : () = []
```

> [!TODO]
> This is a bit inconsitent since `b` is not a keyword here...

And of course we can destruct multiple fields at once:

```s
(let { :a b :c } { :a 1 :b 2 :c 3})
```

```eval
val b : 1 = 1.0
val c : 3 = 3.0
- : () = []
```
