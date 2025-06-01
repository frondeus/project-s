```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

{
  :another 9
})
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  }
}
```

It is returning `b: 5.0` because when reading `root` it reads original value
because left side of the `+` was already calculated.

# Super

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

(thunk () {
  :another 9
  :self (+ 1 (self :another))
  :super (+ 1 (super :another))
}))
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  },
  "self": 10.0,
  "super": 5.0
}
```

# Overriding nested structs

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

{
  :another 9
  :key 10
})
```

```json
{
  "another": 9.0,
  "key": 10.0
}
```

# Deep merging

```
(let left {
    (let x 4)
    :another x
    :key {
      :a 1
      :b (+ 1 (root :another))
    }
  }

(let right (thunk () {
  :another 9
  
  (+obj :key {:c 3})
})

(+ left right)

))
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0,
    "c": 3.0
  }
}
```
