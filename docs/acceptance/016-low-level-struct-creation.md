We will define operations needed for defining low level code
for the constructor
and then based on it write pass rewriting struct definition.


# Empty struct

```
(obj/con (fn (:self :root) (do

  self
)))
```

```json
{}
```


# Inserting field

```
(obj/con (fn (:self :root) (do
  (obj/insert self :key 5)
  self
)))
```

```json
{
  "key": 5.0
}
```

# Inserting two fields


```
(obj/con (fn (:self :root) (do
  (obj/insert self :key 5)
  (obj/insert self :another 6)
  self
)))
```

```json
{
  "another": 6.0,
  "key": 5.0
}
```

# Using macro

```
(obj/condef 
  (obj/insert self :key 5)
  (obj/insert self :another 6)
)
```

```json
{
  "another": 6.0,
  "key": 5.0
}
```

# Nested simple

```
(let :foo (obj/condef
  (obj/insert self :b 4)
  (obj/insert self :a 5)
))

(let :bar (obj/condef
  (obj/insert self :b 10)
  (obj/insert self :c foo)
))

bar
```

```json
{
  "b": 10.0,
  "c": {
    "a": 5.0,
    "b": 4.0
  }
}
```


# Simple root

```
(let :foo (obj/condef
  (obj/insert self :b 4)
  (obj/insert self :a (root :b))
))

foo
```

```json
{
  "a": 4.0,
  "b": 4.0
}
```

Cool.

# Nested root


```
(let :foo (obj/condef
  (obj/insert self :b 4)
  (obj/insert self :a (root :b))
))

(let :bar (obj/condef
  (obj/insert self :b 10)
  (obj/insert self :c foo)
))

bar
```

```json 
{
  "b": 10.0,
  "c": {
    "a": 10.0,
    "b": 4.0
  }
}
```

# Adding objects

```
(+
    (obj/condef
        (obj/insert self :a 1)
        (obj/insert self :b 2)
    )
    (obj/condef
        (obj/insert self :c 3)
    )
)
```


```json
{
  "a": 1.0,
  "b": 2.0,
  "c": 3.0
}
```

## Use self

Self always points to the local object.

```
(+
    (obj/condef
        (obj/insert self :a 1)
        (obj/insert self :b (+ (self :a) 1))
    )
    (obj/condef
        (obj/insert self :c 3)
    )
)
```

```json
{
  "a": 1.0,
  "b": 2.0,
  "c": 3.0
}
```

## Nested structs

Adding nested structs overrides

```
(+
    (obj/condef
        (obj/insert self :a 1)
        (obj/insert self :b (obj/condef
            (obj/insert self :c 2)
        ))
    )
    (obj/condef
        (obj/insert self :b (obj/condef
            (obj/insert self :d 3)
        ))
    )
)
```

```json
{
  "a": 1.0,
  "b": {
    "d": 3.0
  }
}
```

## Super

Super always points to the left side of `+` no matter if it was nested
or not.

```
(+
    (obj/condef
        (obj/insert self :a 1)
    )
    (obj/condef
        (obj/insert self :a (+ (super :a) 1))
    )
)
```

```json
{
  "a": 2.0
}
```

Even though right side modifies self.a, super has an access to :a 1

```
(+
    (obj/condef
        (obj/insert self :a 1)
    )
    (obj/condef
        (obj/insert self :a 3)
        (obj/insert self :b (+ (super :a) 1))
    )
)
```

```json 
{
  "a": 3.0,
  "b": 2.0
}
```

### Nested

```
(obj/condef
    (obj/insert self :a 1)
    (obj/insert self :b (+
        (obj/condef
            (obj/insert self :a 2)
        )
        (obj/condef
            (obj/insert self :a (+ (super :a) 1))
        )
    ))
)
```

```json 
{
  "a": 1.0,
  "b": {
    "a": 3.0
  }
}
```

## Root

### Left

#### Not nested

```
(+
    (obj/condef 
        (obj/insert self :a 1)
        (obj/insert self :b (root :a))
    )
    (obj/condef
        (obj/insert self :d 3)
    )
)
```

```json
{
  "a": 1.0,
  "b": 1.0,
  "d": 3.0
}
```