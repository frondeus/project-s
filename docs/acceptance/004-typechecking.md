# Statically

## Primitives

```
5
```

```type
5
```

```
"41"
```

```type
"41"
```

## Let expressions

```
(let :x 5)
x
```

```type
5
```

## If else

```
(let :x (if true 5 "foo"))

x
```

```type
5 | "foo"
```

## If else the same type 

```
(let :x (if true 5 3))

x
```

```type
5 | 3
```

## If only

```
(let :x (if true 5))

x
```

```type
5 | ()
```

## Functions

```
(fn (:x) x)
```

```type
(Any) -> Any
```

```
(fn (:x :y) (
    if x y
))
```

```type
(bool, Any) -> Any | ()
```

```
(let :f (fn (:x) x))

(f 5)
f
```

```type
(Any) -> Any
```

## Functions with ifs

```
(let :f (fn (:x :y) (if x y)))

(f true "3")

f
```

```type
(bool, Any) -> Any | ()
```

Else the same type

```
(let :f (fn (:x :y) (if x y "foo")))

(f true "3")

f
```

```type
(bool, Any) -> Any | "foo"
```



Else different type 

```
(let :f (fn (:x :y) (if x y "foo")))

(f true 3)

f
```

```type
(bool, Any) -> Any | "foo"
```

# Dynamically:

```
(is-type 5 Number)
```

```json ignore
true
```

```
(is-type 5 String)
```

```json ignore
false
```

```
(is-type "5" Number)
```

```json ignore
false
```

```
(is-type "5" String)
```

```json ignore
true
```
