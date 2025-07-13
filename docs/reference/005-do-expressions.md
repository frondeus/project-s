Sometimes we want to evaluate multiple expressions
but return only the result of the last one.

To do so S-lang provides another special form: `do`

```s
(let :x (do
   1 2 3
))
```

```eval
val x : 3 = 3.0
- : () = []
```

`do` expression has one extra, very important feature - it defines a new lexical scope for variables:

```s
(let :x (do
   (let :y 1)
   2
))
```

```eval
val x : 2 = 2.0
- : () = []
```

Even though we defined `y`, it is not accessible outside the `do` expression. We can use it only inside of it:


```s
(let :x (do
   (let :y 1)
   y
))
```

```eval
val x : 1 = 1.0
- : () = []
```
