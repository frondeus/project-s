So far every example was contained in one file.

This is not ideal for anything serious.

S-lang supports modules and importing.

We will describe them now, starting from the latter:

# Importing

In order to import one file into another, we use the `import` special form:

```s
(let :a (import "a.s"))
```

```a.s
{
   :foo 1
   :bar "2"
}
```


```eval
val a : {foo: 1, bar: "2"} = {
  "bar": "2",
  "foo": 1.0
}
- : () = []
```

By default, exported is the last expression in the file.
In most cases (like in the case above), we import a value of the file as a variable binding.

Of course we can use destructing to automatically add imported object into scope:

```s
(let { :foo :bar } (import "a.s"))
```

```a.s
{
   :foo 1
   :bar "2"
}
```

```eval
val foo : 1 = 1.0
val bar : "2" = "2"
- : () = []
```

However, there is one problem with such approach:
What if we want to export polymorphic function?

For example we created an `id` function in one file, and we want to import it into another:

```s
(let { :id } (import "id.s"))

(id 1)
```

```id.s
(let :id (fn (:x) x))

{ :id id }
```

```eval
val id : ('a) → 1 = "<Function: LispFn>"
- : 1 = 1.0
```

Oh no. The id function imported do not have `('a) -> 'a` type, and the binding is not polymorphic!

The problem is, once we put `:id` function into object `{ :id id }`, we instantiate that function.

In order to solve this issue, S-lang provides another value: A module.

# Modules

To define a module we use `module` special form.
It acts similarly to `do`, creating a new scope but the return value is a module that contains all bindings defined in the scope.

```s
(module
    (let :id (fn (:x) x))
    (let :five 5)
)
```

```eval
- : module {def id: ('a) → 'a, def five: 5} = {
  "five": 5.0,
  "id": "<Function: LispFn>"
}
```

This allows us to import polymorphic function:


```s
(let :id-mod (import "id.s"))

((id-mod :id) 1)
```

```id.s
(module
    (let :id (fn (:x) x))
)
```

```eval
val id-mod : module {def id: ('a) → 'a} = {
  "id": "<Function: LispFn>"
}
- : 1 = 1.0
```

Great!

# Destructing modules

Of course we can use destructing to access module fields:


```s
(let { :id } (import "id.s"))

(id 1)
```

```id.s
(module
    (let :id (fn (:x) x))
)
```

```eval
val id : ('a) → 1 = "<Function: LispFn>"
- : 1 = 1.0
```

> [!TODO]
> Destructing modules instantiates bindings :(
