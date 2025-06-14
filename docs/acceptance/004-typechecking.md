# Statically

## Primitives

```
5
```

```type
Number
```

```
"41"
```

```type
String
```

## Let expressions

```
(let :x 5)
x
```

```type
Number
```

## If else

```
(let :x (if true 5 "foo"))

x
```

```type
String | Number
```

## If else the same type 

```
(let :x (if true 5 3))

x
```

```type
Number
```

## If only

```
(let :x (if true 5))

x
```

```type
Number
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
(Bool, Any) -> Any
```

```
(let :f (fn (:x) x))

(f 5)
f
```

```type
(Number) -> Number
```

## Functions with ifs

```
(let :f (fn (:x :y) (if x y)))

(f true "3")

f
```

```type
(Bool, String) -> String
```

Else the same type

```
(let :f (fn (:x :y) (if x y "foo")))

(f true "3")

f
```

```type
(Bool, String) -> String
```



Else different type 

```
(let :f (fn (:x :y) (if x y "foo")))

(f true 3)

f
```

```type
(Bool, Number) -> String | Number
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
