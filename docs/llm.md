# Lambda Lifting Thunk Test

Testing that thunks inside functions should NOT capture function parameters as free variables:

```s
(fn (:x) (thunk () x))
```

```processed auto-approve
(top-level
  (fn
    (:x)
    (thunk
      (x)
      x)))
```

```eval auto-approve
- : ('a) → 'a = "<Function: LispFn>"
```

Simple test to verify the issue:

```s
(let :f (fn (:x) (thunk () x)))
(let :my-thunk (f 42))
my-thunk
```

```processed auto-approve
(top-level
  (let
    :f
    (fn
      (:x)
      (thunk
        (x)
        x)))
  (let
    :my-thunk
    (f
      42))
  my-thunk)
```

```eval auto-approve
val f : forall ('a) → 'a = "<Function: LispFn>"
val my-thunk : 42 = 42.0
- : 42 = 42.0
```

Test with self parameter specifically:

```s
(fn (:self) (thunk () self))
```

```processed auto-approve
(top-level
  (fn
    (:self)
    (thunk
      (self)
      self)))
```

```eval auto-approve
- : ('a) → 'a = "<Function: LispFn>"
```


```s
(top-level
  (let
    :f
    (cl (:x) ()
      (thunk (x) x)))
  (let
    :my-thunk
    (f
      42))
  my-thunk)
```

```eval auto-approve
val f : forall ('a) → 'a = "<Function: LispFn>"
val my-thunk : 42 = 42.0
- : 42 = 42.0
```
