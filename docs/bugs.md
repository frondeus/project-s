# Types

```s
(let :f (fn (:x) (if true 1 x)))

f
```

```type ignore
(Any) -> Any | 1
```

```
(let :g ((fn () (fn (:x) x))))

(thunk (g) (g 1))

g
```

```type ignore
(1) -> 1
```
