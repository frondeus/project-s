```
(let :x 2)
(+ x x)
```

let {name} = {value} in {body}

```json
4.0
```

You can also use let in structs

```
(struct 
  (let :x 5)
  :key (+ 1 x)
  :another '(+ 1 x)
)
```

```json
{
  "another": "(+ 1 x)",
  "key": 6.0
}
```


# Recursive let

Let's imitate overlays from Nix, without using `+` operator:

```nix
nix-repl> fix = f: let result = f result; in result
nix-repl> pkgs = self: { a = 3; b = 4; c = self.a+self.b; }
nix-repl> fix pkgs
{ a = 3; b = 4; c = 7; }
```


Introducing `let*` which is a recursive `let`.

```
(let :fix (fn (:f) (do
    (let* :result (f result))
    result
)))

(let :pkgs (fn (:this) { # self is a keyword, lets use `this` instead.
    :a 3
    :b 4
    :c (+ (this :a) (this :b))
}))

(fix pkgs)
```

```json
{
  "a": 3.0,
  "b": 4.0,
  "c": 7.0
}
```

# Typechecking

```
(let :x 5)
x
```

```type
number
```