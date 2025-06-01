Primitive atoms are evaluating to themselves

```
(quote 1)
```

```json
1.0
```

```
(quote "42")
```

```json
"42"
```

Symbols are evaluating to themselves as well

```
(quote quote)
```

```json
"quote"
```

Any advanced sexp is just kept as is.

```
(quote (1 2 3))
```

Printing to JSON is wrapping sexp in Strings
```json
"(1 2 3)"
```


We can also use `'` syntax sugar

```
'(1 2 3)
```

```json
"(1 2 3)"
```