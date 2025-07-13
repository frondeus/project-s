S-lang is expression based language.

The most basic expression is a primitive literal.

# Numbers

Numbers are primitive literals that are representing floats.

```s
4
```

```eval
- : 4 = 4.0
```

As you can see, the type of `4` is not `number` but `4`. This is similar to Typescript where literals have singleton types that are a subtype of the primitive types.

So in our example `4 <: number` - a 4 is a subtype of number. Whenever `number` is expected, `4` can be used. However if `4` is expected, then only `4` can be used.

# Strings

S-lang supports the most basic string ever, without escaping, multilines etc.
It's so basic it eats unsalted cooked chicken for a dinner.

```s
"Hello, World!"
```

```eval
- : "Hello, World!" = "Hello, World!"
```

# Booleans

Booleans are primitive literals that are representing true or false.

```s
true
```

```eval
- : true = true
```

```s
false
```

```eval
- : false = false
```

# Keywords

There is a special literal that is representing a keyword. Keywords are symbols that are used to represent record fields, flags, and other special identifiers.

All keywords are prefixed with `:`. In contrast to symbols (defined later), keywords do not evaluate to environment variables.
When transforming s-value into JSON, keywords are represented as strings.

```s
:foo
```

```eval
- : :foo = ":foo"
```
