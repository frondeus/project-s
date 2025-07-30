So far we have covered primitive types, tuples, lists, records, and references.
These give us quite a lot of expressive power, but we're still missing one crucial piece: **sum types**.

Sum types, also known as **enums** or **algebraic data types (ADTs)**, allow us to represent data that can be one of several different variants. Each variant can carry associated data of different types.

# Creating Enums

To create an enum value, we use the `enum` special form:

```s
(enum :circle { :radius 5 })
```

```eval
- : enum  {circle: ({radius: 5})} = {
  "circle": {
    "radius": 5.0
  }
}
```

Here we created an enum with variant `:circle` that carries a record with a `:radius` field.

We can create different variants of the same conceptual enum:

```s
(enum :rectangle { :width 4 :height 3 })
```

```eval
- : enum  {rectangle: ({width: 4, height: 3})} = {
  "rectangle": {
    "height": 3.0,
    "width": 4.0
  }
}
```

# Pattern Matching

The real power of enums comes with **pattern matching** using the `match` special form. This allows us to handle different variants differently:

```s
(let :shape (enum :circle { :radius 5 }))

(match shape
    (:circle :v)
       (* (v :radius) (v :radius) 3.14)
    (:rectangle :v)
       (* (v :width) (v :height))
)
```

```eval
val shape : enum  {circle: ({radius: 5})} = {
  "circle": {
    "radius": 5.0
  }
}
- : number = 78.5
```

In this pattern, `:circle :v` means "if the variant is `:circle`, bind the associated data to variable `v`".

# Functions with Enum Parameters

We can write polymorphic functions that work with enums containing multiple variants:

```s
(let :area (fn (:shape)
    (match shape
        (:circle :v)
           (* (v :radius) (v :radius) 3.14)
        (:rectangle :v)
           (* (v :width) (v :height))
    )
))

(area (enum :circle { :radius 5 }))
```

```eval
val area : forall (enum  {circle: ((:radius) -?-> number) | rectangle: ((:width) -?-> number ∧ (:height) -?-> number)}) → number = "<Function: LispFn>"
- : number = 78.5
```

Notice how the type system infers that `area` accepts an enum with either a `:circle` variant (containing something with a `:radius` field) or a `:rectangle` variant (containing something with both `:width` and `:height` fields).

Let's try it with a rectangle:

```s
(let :area (fn (:shape)
    (match shape
        (:circle :v)
           (* (v :radius) (v :radius) 3.14)
        (:rectangle :v)
           (* (v :width) (v :height))
    )
))

(area (enum :rectangle { :width 4 :height 3 }))
```

```eval
val area : forall (enum  {circle: ((:radius) -?-> number) | rectangle: ((:width) -?-> number ∧ (:height) -?-> number)}) → number = "<Function: LispFn>"
- : number = 12.0
```

# Type Safety

One of the key benefits of enums is type safety. The type system will catch errors when you try to use variants that don't exist:

```s
(let :area (fn (:shape)
    (match shape
        (:circle :v)
           (* (v :radius) (v :radius) 3.14)
        (:rectangle :v)
           (* (v :width) (v :height))
    )
))

(area (enum :triangle { :bottom 4 :height 2 }))
```

```eval
Error: Missing variant 'triangle'
    ╭─[ <input>:10:7 ]
    │
  2 │ ╭─▶     (match shape
    ┆ ┆   
  7 │ ├─▶     )
    │ │           
    │ ╰─────────── Expected here
    │ 
 10 │     (area (enum :triangle { :bottom 4 :height 2 }))
    │           ────────────────────┬───────────────────  
    │                               ╰───────────────────── Used here
────╯

```

The type system will also catch errors when the associated data has the wrong shape:

```s
(let :area (fn (:shape)
    (match shape
        (:circle :v)
           (* (v :radius) (v :radius) 3.14)
        (:rectangle :v)
           (* (v :width) (v :height))
    )
))

(area (enum :circle { :radus 2 }))
```

```eval
Error: Undefined field: radius
    ╭─[ <input>:4:15 ]
    │
  4 │            (* (v :radius) (v :radius) 3.14)
    │               ─────┬─────  
    │                    ╰─────── Used here
    │ 
 10 │ (area (enum :circle { :radus 2 }))
    │                     ──────┬─────  
    │                           ╰─────── Record defined here
────╯
Error: Undefined field: radius
    ╭─[ <input>:4:27 ]
    │
  4 │            (* (v :radius) (v :radius) 3.14)
    │                           ─────┬─────  
    │                                ╰─────── Used here
    │ 
 10 │ (area (enum :circle { :radus 2 }))
    │                     ──────┬─────  
    │                           ╰─────── Record defined here
────╯

```

Notice how the error points out that `:radius` is expected but `:radus` (with a typo) was provided.

# Ascriptions

Just like other types, we can explicitly type enums:

```s
(let :shape (:
    (enum :circle {:radius :number}
          :rectangle {:width :number :height :number }
    )
    (enum :circle { :radius 5 })
))
```

```eval
val shape : enum  {circle: ({radius: number}) | rectangle: ({width: number, height: number})} = {
  "circle": {
    "radius": 5.0
  }
}
- : () = []
```

# Enums with Different Data Types

Enum variants don't have to carry records - they can carry any type of data:

```s
(let :result (enum :success "Operation completed"))
```

```eval
val result : enum  {success: ("Operation completed")} = {
  "success": "Operation completed"
}
- : () = []
```

```s
(let :result (enum :error 404))
```

```eval
val result : enum  {error: (404)} = {
  "error": 404.0
}
- : () = []
```

We can match on these different types:

```s
(let :handle-result (fn (:result)
    (match result
        (:success :msg)
            (print "Success: " msg)
        (:error :code)
            (print "Error " code)
    )
))

(handle-result (enum :success "All good"))
```

```eval
val handle-result : forall (enum  {success: ('a) | error: ('b)}) → number = "<Function: LispFn>"
- : number = 1.0
```

# Enums without Associated Data

Some variants might not need to carry any data.

```s
(let :status (enum :pending))
```

```eval
val status : enum  {pending: ()} = {
  "pending": null
}
- : () = []
```

```s
(let :status (enum :ready))
```

```eval
val status : enum  {ready: ()} = {
  "ready": null
}
- : () = []
```

# Destructing enums

During match expression we can always destruct associated data:


```s
(let :area (fn (:shape)
    (match shape
        (:Circle { :radius })
           (* radius radius 3.14)
        (:Rectangle { :width :height })
           (* width height)
    )
))

(area (enum :Circle { :radius 5 }))
```

```eval
val area : forall (enum  {Circle: ({radius: number}) | Rectangle: ({width: number, height: number})}) → number = "<Function: LispFn>"
- : number = 78.5
```

# Multiple associated data

We are not limited only to 0-1 associated data.
If we want, we can use enums like this:


```s
(let :area (fn (:shape)
    (match shape
        (:Circle :radius)
           (* radius radius 3.14)
        (:Rectangle :width :height)
           (* width height)
    )
))

{
   :a (area (enum :Rectangle 5 10))
   :b (area (enum :Circle 5))
}
```

```eval
val area : forall (enum  {Circle: (number) | Rectangle: (number, number)}) → number = "<Function: LispFn>"
- : {a: number, b: number} = {
  "a": 50.0,
  "b": 78.5
}
```
