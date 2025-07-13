By default S is statically typed language with global type inference based on Algebraical Subtyping paper (Dolan & Mycroft, 2016).

However, sometimes we need to specify the type of a value explicitly - as a form of documentation or to help the type inferer.

Ascriptions are used by special form `(: <type> <value>)`.

```s
(let :x (: 5 5))
```

```eval
val x : 5 = 5.0
- : () = []
```

Now, with ascriptions we can return to the primitives and talk about general types:

## Numbers

```s
(let :x (: :number 5))
```

```eval
val x : number = 5.0
- : () = []
```

## Strings

```s
(let :x (: :string "Hello, World!"))
```

```eval
val x : string = "Hello, World!"
- : () = []
```

## Booleans

```s
(let :x (: :bool true))
```

```eval
val x : bool = true
- : () = []
```

## Keywords

```s
(let :x (: :keyword :foo))
```

```eval
val x : keyword = ":foo"
- : () = []
```
