Basic struct construction

```
(struct (quote (
    :name "Name"
    :surname "Surname"
)))
```

```json
{
  "name": "Name",
  "surname": "Surname"
}
```

Quote syntax sugar:
```
(struct '(
    :name "Name"
    :surname "Surname"
))
```

```json
{
  "name": "Name",
  "surname": "Surname"
}
```


Building with quasiquotes

```
(struct `(
  :name "John Smith"
  :age ,(+ 20 3)
))
```

```json
{
  "age": 23.0,
  "name": "John Smith"
}
```

# Accessing struct

```
( 
  (struct '(:key 1 :another 2))
  :another
)
```

```json
2.0
```

Or when struct is named

```
(let :foo (struct '(:key 1 :another 2))
  (foo :another)
)
```

```json
2.0
```

# Self

Let's say that for now keys HAVE TO
be ordered explicitly

```
(struct `(
  :another (+ 1 1)
  :key (+ 1 (self :another))
))
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
(struct `(
  :another 4
  :key (struct '(
    :a 1
    :b (+ 1 (root :another))
  ))
))
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
}
  (has? x :key)
)
```

```json
true
```

```
(let :x {
  :key 42.0
}
  (has? x :another)
)
```

```json
false
```
