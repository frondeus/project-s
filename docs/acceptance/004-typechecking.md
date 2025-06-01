Basic type inference (stub)

```
5
```

Statically

```type
Number
```

```
"41"
```

```type
String
```

Dynamically:

```
(is-type 5 Number)
```

```json
true
```

```
(is-type 5 String)
```

```json
false
```

```
(is-type "5" Number)
```

```json
false
```

```
(is-type "5" String)
```

```json
true
```
