Sometimes it is useful to return an error.
Errors are a special type of value that store a string with the message:

```s
(error "message")
```

```eval
- : error = "<Error: message>"
```

Type `error` acts like `Bottom` type known also as `Never` type.
Sure, it is possible to construct an error (which is not possible with `Bottom`).
But the idea is - during the development, you can create an error (which basically acts like a poor-man's-exception or a panic) but you dont have to be additionally punished by type error ("expected number, but got error instead").
So whenever `x` is expected in a type system, error type is sufficient.
