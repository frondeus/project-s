# Declaring a function
```
(fn (:x :y) (+ x y))
```

```json
"<Function: LispFn>"
```

```
(fn (:x) x)
```

```type
('a) → 'a
```

# Calling a function

```
((fn (:x :y) (+ x y)) 1 2)
```

```json
3.0
```

```
(let :f (fn (:x) x))

(f 4)
```

```type
4
```

```
(let :f (fn (:x) x))

(f 5)
f
```

```type
('a) → 'a
```

# Closures

Function is automatically transformed into a closure when necessary
It captures the context by copying it. There are no references


```
(let :top (
  fn () (do 
    (let :c 42.0)
    (fn (:a :b) (+ a b c))
  )
))

((top) 1.0 2.0)
```

```json
45.0
```

## Capturing `self`


```
(({
  :a 42.0
  :b (fn () (self :a))
} :b))
```

```json
42.0
```

## Capturing `root`

```
((({
  :a 42.0
  :b {
    :c (fn () (root :a))
  }
} :b) :c))
```

```json
42.0
```

## Capturing `super`

```
(( (+ {
  :a 42.0
}

{
  :a (fn () (+ (super :a) 10.0))
}) :a))
```

```json
52.0
```
