Explicit keyword usage

```
(quasiquote (1 2 (+ 1 2)))
```

```json
"(1 2 (+ 1 2))"
```

## Using of `unquote`


```
(quasiquote (1 2 (unquote (+ 1 2))))
```

```json
"(1 2 3)"
```

Reader shortcut.

```
`(1 2 ,(+ 1 2))
```

```json
"(1 2 3)"
```
