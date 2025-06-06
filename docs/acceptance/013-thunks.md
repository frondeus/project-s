# Explicit thunk creation

```
(thunk () 123)
```

```json
"<Thunk: Thunk { inner: RefCell { value: ToEvaluate { captured: {}, body: SExpId { id: 4, generation: 0 } } } }>"
```

```
(let :x 42.0 )
  (thunk (x) (+ 123 x))
```

```json
"<Thunk: Thunk { inner: RefCell { value: ToEvaluate { captured: {\"x\": Number(42.0)}, body: SExpId { id: 9, generation: 0 } } } }>"
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