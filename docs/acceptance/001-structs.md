Basic struct construction

```
(struct 
    :name "Name"
    :surname "Surname"
)
```

```json
{
  "name": "Name",
  "surname": "Surname"
}
```

# Accessing struct

```
( 
  (struct :key 1 :another 2)
  :another
)
```

```json
2.0
```

Or when struct is named

```
(let :foo (struct :key 1 :another 2))
(foo :another)
```

```json
2.0
```

# Self

Let's say that for now keys HAVE TO
be ordered explicitly

```
(struct
  :another (+ 1 1)
  :key (+ 1 (self :another))
)
```

```json
{
  "another": 2.0,
  "key": 3.0
}
```

# Root

To access top object

```
(struct 
  :another 4
  :key (struct 
    :a 1
    :b (+ 1 (root :another))
  )
)
```

```json
{
  "another": 4.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  }
}
```

# Reader sugar

```
{
  (let :x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}
```

```json
{
  "another": 4.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  }
}
```

# has? function

```
(let :x {
  :key 42.0
})
  (has? x :key)
```

```json
true
```

```
(let :x {
  :key 42.0
})
  (has? x :another)
```

```json
false
```


# Differences between `self` `super` `root` and `origin`

## Root

```
{
  :a "the most top a"
  :b (+ {
    :d "left side d"
    :a "left side a"
  }
  {
    :a "the most top a from right side"
    :b {
      :a "the most inner a from right side"
      :c (root :a)
    }
  )
}
```

```json
{
  "a": "the most top a",
  "b": {
    "a": "the most top a from right side",
    "b": {
      "a": "the most inner a from right side",
      "c": "the most top a from right side"
    },
    "d": "left side d"
  }
}
```

## Super

```
{
  :a "the most top a"
  :b (+ {
    :d "left side d"
    :a "left side a"
  }
  {
    :a "the most top a from right side"
    :b {
      :a "the most inner a from right side"
      :c (super :a)
    }
  )
}
```

```json
{
  "a": "the most top a",
  "b": {
    "a": "the most top a from right side",
    "b": {
      "a": "the most inner a from right side",
      "c": "left side a"
    },
    "d": "left side d"
  }
}
```

## Self 

```
{
  :a "the most top a"
  :b (+ {
    :d "left side d"
    :a "left side a"
  }
  {
    :a "the most top a from right side"
    :b {
      :a "the most inner a from right side"
      :c (self :a)
    }
  )
}
```

```json
{
  "a": "the most top a",
  "b": {
    "a": "the most top a from right side",
    "b": {
      "a": "the most inner a from right side",
      "c": "the most inner a from right side"
    },
    "d": "left side d"
  }
}
```

## Origin


```
{
  :a "the most top a"
  :b (+ {
    :d "left side d"
    :a "left side a"
  }
  {
    :a "the most top a from right side"
    :b {
      :a "the most inner a from right side"
      :c (origin :a)
    }
  )
}
```

```json
{
  "a": "the most top a",
  "b": {
    "a": "the most top a from right side",
    "b": {
      "a": "the most inner a from right side",
      "c": "the most top a"
    },
    "d": "left side d"
  }
}
```

