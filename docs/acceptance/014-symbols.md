```
symbol
```

```cst
Symbol("symbol")
```

this evaluates immediately to its variable

```json
"<Error: Undefined variable: symbol>"
```

```
'symbol
```

```cst
List([Symbol("quote"), Symbol("symbol")])
```

This is quoted

```json
"symbol"
```

```
:symbol
```

```cst
Keyword("symbol")
```

This as well

```json
":symbol"
```