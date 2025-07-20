The most basic form of control flow is If expression.

Note, it is still an expression, not a statement.


# One branch expression

It is valid to create an if expression with only one branch:

```s
(let :x (if true 4))
```

Of course, for the type inferer condition may be anything, so it has to know what would be the type of `:x` in case of `false`. Since we did not provide a `else` branch, the type is by default `()` - Unit.


```eval
val x : 4 ∨ () = 4.0
- : () = []
```

As you can see, the type of `:x` is `4 ∨ ()` - which you can read as "it is either `4` or `()`.

# If else

However if we provide an `else` branch:

```s
(let :x (if true 4 5))
```

```eval
val x : 4 ∨ 5 = 4.0
- : () = []
```

You could expect to see `x : number` instead. But at that point type inferer
is not sure if that's what you want. For example in other case you could have:

```s
(let :x (if true "yes" "no"))
```

```eval
val x : "yes" ∨ "no" = "yes"
- : () = []
```

In that case having a type `"yes" v "no"` gives more meaning than `string`.

What if we ascribe both branches?

```s
(let :yes (: :string "yes"))
(let :no  (: :string "no"))
(if true yes no)
```

> [!TODO]
> This should probably be `- : string` instead of `- : string ∨ string`

> [!TODO]
> Modules should use IndexMap instead of BTree

```eval
val yes : string = "yes"
val no : string = "no"
- : string ∨ string = "yes"
```
