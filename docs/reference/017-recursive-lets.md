Up to this point we were working only with plain let expressions.

Now it's time to introduce recursive lets.

Recursive let expressions allow us to define more interesting concepts.

In order to use recursive let, one has to use `let*` special form.

```s
(let* :f
    (fn (:n)
        (if (= n 0)
            1
            (* n
                (f
                    (- n 1)
                )
            )
        )
    )
)

(f 5)
```

```eval
val f : forall ('a ∧ number ∧ number) → 1 ∨ number = "<Function: LispFn>"
- : 1 ∨ number = 120.0
```

We can of course define the type explicitly:

```s
(let* :f (:
    (fn (:number) :number)
    (fn (:n)
        (if (= n 0)
            1
            (* n
                (f
                    (- n 1)
                )
            )
        )
    )
))

(f 5)
```

```eval
val f : forall (number) → number = "<Function: LispFn>"
- : number = 120.0
```


Comparing to classic `let`, recursive one allows to define multiple bindings at once, and use them mutually.

```s
(let* :is-odd (fn (:n)
        (if (= n 0)
            false
            (is-even (- n 1))
        )
      )
      :is-even (fn (:n)
        (if (= n 0)
            true
            (is-odd (- n 1))
        )
      )
)

(is-odd 5)
```

```eval
val is-even : forall ('a ∧ number) → true ∨ false = "<Function: LispFn>"
val is-odd : forall ('a ∧ number) → false ∨ true = "<Function: LispFn>"
- : false ∨ true = true
```

# Self-referencing records

Thanks to our newest invention - recursion, we can express more interesting concept - a fixpoint function.

```s
(let :fix (fn (:f) (do
    (let* :result (f result))
    result
)))
```

```eval
val fix : forall (('a) -?-> 'a) → 'b = "<Function: LispFn>"
- : () = []
```

Let's think what we do here - we introduce recursive `:result`, that calls `f` with itself as an argument.
But why? Wouldn't that cause infinite loop?

No if you are lazy enough ;-) (Yes, I'm refering to [[007-laziness.md]]).

The main usage is to define a record that in one of fields is referencing another field from the same record.
To do so, we wrap that record in a function, that takes `:self` as a parameter:

```s
(let :obj  (fn (:self) {
    :a 1
    :b 2
    :c (+ (self :a) (self :b) )
}))
```

```eval
val obj : forall ((:a) -?-> number ∧ (:b) -?-> number) → {a: 1, b: 2, c: number} = "<Function: LispFn>"
- : () = []
```

Now, we (hopefully) should be able to use our `fix` function to create a final record:

```s
(let :fix (fn (:f) (do
    (let* :result (f result))
    result
)))

(let :obj  (fn (:self) {
    :a 1
    :b 2
    :c (+ (self :a) (self :b) )
}))

(fix obj)
```

```eval
val fix : forall (('a) -?-> 'a) → 'b = "<Function: LispFn>"
val obj : forall ((:a) -?-> number ∧ (:b) -?-> number) → {a: 1, b: 2, c: number} = "<Function: LispFn>"
- : {a: 1, b: 2, c: number} = "<Error: Thunk is already evaluating>"
```

Thunk is already evaluating?
Well yes. Every binding created with recursive let (`:result` in that case) is created as a thunk.
That's how S-lang is adding "something" to the binding before that something is evaluated.

In that case, we are evaluating `fn (:self)` call, so we build a record, and while building a record,
we evaluate `(self :a)`...

But `self` is not yet defined, as we are currently in the process of building it!.

Therefore to make it work, we need to wrap the value of `:c` into a thunk.



```s
(let :fix (fn (:f) (do
    (let* :result (f result))
    result
)))

(let :obj  (fn (:self) {
    :a 1
    :b 2
    :c (thunk () (+ (self :a) (self :b) ))
}))

(fix obj)
```

```eval
val fix : forall (('a) -?-> 'a) → 'b = "<Function: LispFn>"
val obj : forall ((:a) -?-> number ∧ (:b) -?-> number) → {a: 1, b: 2, c: number} = "<Function: LispFn>"
- : {a: 1, b: 2, c: number} = {
  "a": 1.0,
  "b": 2.0,
  "c": 3.0
}
```

But, how does it work?
When we evaluate `:result`, first we call `f` with `Thunk/Evaluating` special value.

`f` returns `{ :a 1 :b 2 :c (Thunk/ToEvaluate) }` and that is assigned to `:result`.

However, `Thunk/ToEvaluate` has caught the `:result` into captured variables.
So when we finally print the struct, `:c` is evaluated with recursive `self`, and that finally can work :)

# Overlays

With the last trick we could do step further and introduce overlays from Nix language.
But type-safe ;-)

```s
(let :extend (fn (:base :ext) (do
   (let*
        :super (base result {})
        :result (ext result super)
   )
   result
)))

(let :base (fn (:self :super) { :a 1 :b 2 }))

(let :obj  (fn (:self :super) (obj/extend super
    :c (thunk () (+ (super :a) (super :b) ))
)))

(extend base obj)
```

```eval
val base : forall ('a, 'b) → {a: 1, b: 2} = "<Function: LispFn>"
val extend : forall (('a, {}) -?-> 'b, ('c, 'b) -?-> 'a ∧ 'c) → 'd = "<Function: LispFn>"
val obj : forall ('a, (:a) -?-> number ∧ (:b) -?-> number) → 'b extends {c: number} = "<Function: LispFn>"
- : {a: 1, b: 2, c: number} = {
  "a": 1.0,
  "b": 2.0,
  "c": 3.0
}
```
