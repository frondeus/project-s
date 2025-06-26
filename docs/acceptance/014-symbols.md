```
symbol
```

```cst
source_file - "symbol"
 symbol - "symbol"

```

this evaluates immediately to its variable

```json
"<Error: Undefined variable: symbol>"
```

```
'symbol
```

```cst
source_file - "'symbol"
 quote - "'symbol"
  ' - "'"
  symbol - "symbol"

```

This is quoted

```json
"symbol"
```

```
:symbol
```

```cst
source_file - ":symbol"
 keyword - ":symbol"

```

This as well

```json
":symbol"
```