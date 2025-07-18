Sometimes there is a need to make a new record from the old one, by adding new fields.

Theoretically you could use destructing to create a new record like this:

```s
(let :old { :a 1 :b 2 })

(let :new (do
    (let { :a :b } old)

    { :a a :b b :c 3 })
)

```

```eval
val new : {a: 1, b: 2, c: 3} = {
  "a": 1.0,
  "b": 2.0,
  "c": 3.0
}
val old : {a: 1, b: 2} = {
  "a": 1.0,
  "b": 2.0
}
- : () = []
```

However, this is a bit tediouso

That's why S-lang has an additional special form `obj/extend`:

```s
(let :old { :a 1 :b 2 })
(let :new (obj/extend old :c 3))
```

```eval
val new : {c: 3, a: 1, b: 2} = {
  "a": 1.0,
  "b": 2.0,
  "c": 3.0
}
val old : {a: 1, b: 2} = {
  "a": 1.0,
  "b": 2.0
}
- : () = []
```
