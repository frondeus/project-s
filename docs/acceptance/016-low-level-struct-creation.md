We will define operations needed for defining low level code
for the constructor
and then based on it write pass rewriting struct definition.


# Empty struct

```
(obj/con (fn (:self :root :origin) (do

  self
)))
```

```json
{}
```


# Inserting field

```
(obj/con (fn (:self :root :origin) (do
  (obj/put :key 5)
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
(obj/con (fn (:self :root :origin) (do
  (obj/put :key 5)
  (obj/put :another 6)
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
  (obj/put :key 5)
  (obj/put :another 6)
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
  (obj/put :b 4)
  (obj/put :a 5)
))

(let :bar (obj/condef
  (obj/put :b 10)
  (obj/put :c foo)
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
  (obj/put :b 4)
  (obj/put :a (root :b))
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
  (obj/put :b 4)
  (obj/put :a (root :b))
))

(let :bar (obj/condef
  (obj/put :b 10)
  (obj/put :c foo)
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
        (obj/put :a 1)
        (obj/put :b 2)
    )
    (obj/condef
        (obj/put :c 3)
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
        (obj/put :a 1)
        (obj/put :b (+ (self :a) 1))
    )
    (obj/condef
        (obj/put :c 3)
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
        (obj/put :a 1)
        (obj/put :b (obj/condef
            (obj/put :c 2)
        ))
    )
    (obj/condef
        (obj/put :b (obj/condef
            (obj/put :d 3)
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
        (obj/put :a 1)
    )
    (obj/condef
        (obj/put :a (+ (super :a) 1))
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
        (obj/put :a 1)
    )
    (obj/condef
        (obj/put :a 3)
        (obj/put :b (+ (super :a) 1))
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
    (obj/put :a 1)
    (obj/put :b (+
        (obj/condef
            (obj/put :a 2)
        )
        (obj/condef
            (obj/put :a (+ (super :a) 1))
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
        (obj/put :a 1)
        (obj/put :b (root :a))
    )
    (obj/condef
        (obj/put :d 3)
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

#### Nested (+ -> {})

```
(+
    (obj/condef
        (obj/put :a 1)
        (obj/put :b (obj/condef
            (obj/put :c (root :a))
        ))
    )
    (obj/condef
        (obj/put :d 3)
    )
)
```

```json
{
  "a": 1.0,
  "b": {
    "c": 1.0
  },
  "d": 3.0
}
```

#### Nested ({} -> +)

```
(obj/condef
    (obj/put :a 1)    
    (obj/put :b (+
        (obj/condef 
            (obj/put :a 2)
            (obj/put :b (obj/condef
                (obj/put :c (root :a))
            ))
        )
        (obj/condef
            (obj/put :d 3)
        )
    ))
)
```

```json
{
  "a": 1.0,
  "b": {
    "a": 2.0,
    "b": {
      "c": 1.0
    },
    "d": 3.0
  }
}
```

### Right

#### Not nested

```
(+
    (obj/condef 
        (obj/put :a 1)
        (obj/put :d 3)
    )
    (obj/condef
        (obj/put :a 2)
        (obj/put :b (root :a))
    )
)
```


```json
{
  "a": 2.0,
  "b": 2.0,
  "d": 3.0
}
```

#### Nested (+ -> {})

```
(+
    (obj/condef
        (obj/put :a 1)
        (obj/put :d 3)
    )
    (obj/condef
        (obj/put :a 2)
        (obj/put :b (obj/condef
            (obj/put :c (root :a))
        ))
    )
)
```

```json
{
  "a": 2.0,
  "b": {
    "c": 2.0
  },
  "d": 3.0
}
```


When right side doesnt have :a
it takes the :a from super side.

```
(+
    (obj/condef
        (obj/put :a 1)
        (obj/put :d 3)
    )
    (obj/condef
        (obj/put :b (obj/condef
            (obj/put :c (root :a))
        ))
    )
)
```

```json
{
  "a": 1.0,
  "b": {
    "c": 1.0
  },
  "d": 3.0
}
```

#### Nested ( {} -> + )

```
(obj/condef
    (obj/put :a 1)
    (obj/put :b (+
        (obj/condef
            (obj/put :d 3)
        )
        (obj/condef
            (obj/put :a 2)
            (obj/put :b (obj/condef
                (obj/put :c (root :a))
            ))
        )
    ))
)
```

```json
{
  "a": 1.0,
  "b": {
    "a": 2.0,
    "b": {
      "c": 2.0
    },
    "d": 3.0
  }
}
```

## Origin

Comparing all others

### Root

```
(obj/condef
    (obj/put :a "the most top a")
    (obj/put :b (+
        (obj/condef
            (obj/put :d "left side d")
            (obj/put :a "left side a")
        )
        (obj/condef
            (obj/put :a "the most top a from right side")
            (obj/put :b (obj/condef
                (obj/put :a "the most inner a from right side")
                (obj/put :c (root :a))
            ))
        )
    ))
)
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

### Super

```
(obj/condef
    (obj/put :a "the most top a")
    (obj/put :b (+
        (obj/condef
            (obj/put :d "left side d")
            (obj/put :a "left side a")
        )
        (obj/condef
            (obj/put :a "the most top a from right side")
            (obj/put :b (obj/condef
                (obj/put :a "the most inner a from right side")
                (obj/put :c (super :a))
            ))
        )
    ))
)
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

### Self

```
(obj/condef
    (obj/put :a "the most top a")
    (obj/put :b (+
        (obj/condef
            (obj/put :d "left side d")
            (obj/put :a "left side a")
        )
        (obj/condef
            (obj/put :a "the most top a from right side")
            (obj/put :b (obj/condef
                (obj/put :a "the most inner a from right side")
                (obj/put :c (self :a))
            ))
        )
    ))
)
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

### Origin


```
(obj/condef
    (obj/put :a "the most top a")
    (obj/put :b (+
        (obj/condef
            (obj/put :d "left side d")
            (obj/put :a "left side a")
        )
        (obj/condef
            (obj/put :a "the most top a from right side")
            (obj/put :b (obj/condef
                (obj/put :a "the most inner a from right side")
                (obj/put :c (origin :a))
            ))
        )
    ))
)
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


# Using ifs

```
(obj/condef
    (if true (obj/put :key 42))
    (if false (obj/put :false 13))
    (if false (obj/put :true 10) (obj/put :else 12))
)
```

```json
{
  "else": 12.0,
  "key": 42.0
}
```
