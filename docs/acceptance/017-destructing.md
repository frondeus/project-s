# Destructing

Lists:

```
(let (:a :b :c) (list 1 2 3))

(+ a b c)
```

```json
6.0
```

```
(let (:a :b :c) (list 1 2 3))

a
```

```type
Number
```

Destructing nested lists

```
(let (:a (:b :c) :d) (list 1 (list 2 3) 4))

(+ a b c d)
```

```json
10.0
```

```
(let (:a (:b :c) :d) (tuple 1 (tuple 2 3) 4))

b
```

```type
Number
```


Destructing objects

Simple name destructing 

```
(let { :a :b :c } { :a 1 :d 4 :b 2 :e 6 :c 3})

(+ a b c)
```

```json
6.0
```

Renaming

```
(let { :a b } { :a 1 :b 2 :c 3})

b
```

```json
1.0
```

Nested patterns

```
(let { :a { :b } :c } { :a { :b 1 } :c 3})

(+ b c)
```


```json
4.0
```




