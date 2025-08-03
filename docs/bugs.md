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


```
(let :f (: (fn (tuple 'a 'b) 'a)
   (fn (:a :b) b)
))

(f 1 2)

f
```

```type ignore
(2 & 1, 2) -> 2 | 1
```


```s
(let :id (fn (:x) x))
# We create monomorphic ID function
(let :mono-id (id (fn (:x) x)))

(let :x (fn (:a) (if a "4" 5)))
(mono-id (x true))
mono-id
```

```type ignore
("4" & 5) -> "4" | 5
```

# Types - missing in prelude

find-or

```s
(let :s (list/find-or
    (list 1 2 4)
    (fn (:x) (= x 2))
    0
    )
)
s
```

```type ignore
4 | 2 | 1 | 0
```
