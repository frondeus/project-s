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

```json-eager
{
  "another": "(+ 1 x)",
  "key": 6.0
}
```
