# Explicit thunk creation

```
(thunk () 123)
```

```json-lazy
"<Thunk: Thunk>"
```

```
(let :x 42.0 )
  (thunk (x) (+ 123 x))
```

```json-lazy
"<Thunk: Thunk>"
```


# Thunk usage

```
(let :x (thunk () 42.0))
  (+ x 1.0)
```

```json
43.0
```

# Thunk caching

```
(
  let :x (thunk () (print 42.0))
)
    (+ x x)

```

```log
Number(42.0)

```

Note, 42 is logged only once.