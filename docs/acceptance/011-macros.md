We can define macros

```
(
  (macro (:x :y) `(+ ,x ,y))
  1 2
)
```

````macro
Error: Macro is forbidden in this context
   ╭─[ <input>:2:3 ]
   │
 2 │   (macro (:x :y) `(+ ,x ,y))
   │   ─────────────┬────────────  
   │                ╰────────────── Used here
───╯

````

````json
Error: Macro is forbidden in this context
   ╭─[ <input>:2:3 ]
   │
 2 │   (macro (:x :y) `(+ ,x ,y))
   │   ─────────────┬────────────  
   │                ╰────────────── Used here
───╯

````

```
(
  let :var (macro (:name :value) `(let ,name ,value))
)
(var :x 4.2)
x
```

```macro
(top-level
  (let
    :var
    ())
  (let
    :x
    4.2)
  x)
```

```json
4.2
```

We can call macros in inside of struct creation

```
(
  let :fif (macro (:name :co :the :els) `(if ,co '(,name ,the) '(,name ,els))))
  {
    (fif :key true 42.0 10.0)
  }
```

```json
{
  "key": 42.0
}
```
