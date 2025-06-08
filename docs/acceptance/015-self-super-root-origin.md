# Root outside of `+`.
In javascript we would define it as:

```js
const { create_obj } = require("./docs/obj");

let foo = create_obj(({self, root}) => {
  self.set("b", 4);
  self.set("a", root.get("b"));
});

let bar = create_obj(({self, root}) => {
  self.set("b", 10);
  self.set("c", foo({root}));
});

({ "bar": bar({}), "foo": foo({}) })
```

```js-eval
{ bar: { b: 10, c: { b: 4, a: 10 } }, foo: { b: 4, a: 4 } }
```

Note - `foo` used outside of object construction - `root` refers to the same pointer as `self`
resulting in `{a: 4, b: 4}`.

But when `foo` was used as a field then `a: 10`!

What's worth pointing - object constructor is not a thunk!
It's a separate being that is not cached.
It is more similar to a function or a closure.

How that would affect `super`? Super is similar. When `root` or `super` is in use
it cannot be expressed as thunk!
Otherwise, it would work only once!

# Adding objects

Adding objects without context is dead simple
```js
const { create_obj, add, output } = require("./docs/obj");
// Adding objects without context
output(add(
  create_obj({
    a: 1,
    b: 2
  }),
  create_obj({
    c: 3
  })
))
```

```js-eval
{
  "a": 1,
  "b": 2,
  "c": 3
}
```

## Use self

Using `self` is also simple - it **always** points to the local object.
```js
const { create_obj, add, output } = require("./docs/obj");
// Use self
output(add(
  create_obj(({self}) => {
    self.set("a", 1);
    self.set("b", self.get("a") + 1);
  }),
  create_obj({
    c: 3
  })
))
```

```js-eval
{
  "a": 1,
  "b": 2,
  "c": 3
}
```

## Nested structs

Adding nested structs overrides

```js
const { create_obj, add, output } = require("./docs/obj");
// Adding nested structs
output(add(
  create_obj(({self, root}) => {
    self.set("a", 1);
    self.set("b", create_obj({
      c: 2
    })({root}));
  }),
  create_obj(({self, root}) => {
    self.set("b", create_obj({
      d: 3
    })({root}))
  })
))
```

```js-eval
{
  "a": 1,
  "b": {
    "d": 3
  }
}
```

Okay. Now There are `root` and `super`. I think `super` will be counterintuitevily easier to explain

## Super


### Not nested

Super always points to the left side of + no matter if it was nested or not
Here it isn't nested

```js
const { create_obj, add, output } = require("./docs/obj");
// Using super
output(
  add(
    create_obj({
      a: 1
    }),
    create_obj(({self, root, super_}) => {
      self.set("a", super_.get("a") + 1)
    })
  )
)
```

```js-eval
{
  "a": 2
}
```

Even though right side modifies `self.a`, `super_` has an access to `a: 1`.

```js
const { create_obj, add, output } = require("./docs/obj");
// Using super (when right side overwrites super field)
output(
  add(
    create_obj({
      a: 1
    }),
    create_obj(({self, root, super_}) => {
      self.set("a", 3);
      self.set("b", super_.get("a") + 1)
    })
  )
)
```

```js-eval
{
  "a": 3,
  "b": 2
}
```


### Nested

But what if the `add` operator is used inside of a field

Here, super.a points to `a: 2` no matter that the `+` was nested.

```js
const { create_obj, add, output } = require("./docs/obj");
// Using super in nested struct
output(create_obj(({self, root}) => {
  self.set("a", 1);
  self.set("b", add(
    create_obj({
      a: 2
    }),
    create_obj(({self, root, super_}) => {
      self.set("a", super_.get("a") + 1);
    })
  )({root}));
}))
```

```js-eval
{
  "a": 1,
  "b": {
    "a": 3
  }
}
```

## Root

So now big question is, how `root` should behave in adding?

### Left

First, let's focus on LEFT side.

#### Not nested

The easiest case is:

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in left side
output(
  add(
    create_obj(({self, root}) => {
      self.set("a", 1);
      self.set("b", root.get("a"));
    }),
    create_obj({
      d: 3
    })
  )
)
```

Here, root points to the same object as self...
```js-eval
{
  "a": 1,
  "b": 1,
  "d": 3
}
```

#### Nested (+ -> {})

But if in the left side we have a nested object?

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in nested left (+ -> {})
output(
  add(
    create_obj(({self, root}) => {
      self.set("a", 1);
      self.set("b", create_obj(({self, root}) => {
        self.set("c", root.get("a"));
      })({root}))
    }),
    create_obj({
      d: 3
    })
  )
)
```

"c" points to "a". Makes sense.
```js-eval
{
  "a": 1,
  "b": {
    "c": 1
  },
  "d": 3
}
```

#### Nested ({} -> +)

Ok, but now.... If that add operator is inside of struct?

Root points to "1", not "2".

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in nested left ({} -> +)
output(
  create_obj(({self, root}) => {
    self.set("a", 1);
    self.set("b", add(
      create_obj(({self, root}) => {
        self.set("a", 2);
        self.set("b", create_obj(({self, root}) => {
          self.set("c", root.get("a"));
        })({root}))
      }),
      create_obj({
        d: 3
      })
    )({root}));

  })
)
```

```js-eval
{
  "a": 1,
  "b": {
    "a": 2,
    "b": {
      "c": 1
    },
    "d": 3
  }
}
```

### Right

Okay. What about right side?

#### Not nested

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in left side
output(
  add(
    create_obj({
      a: 1,
      d: 3,
    }),
    create_obj(({self, root}) => {
      self.set("a", 2);
      self.set("b", root.get("a"));
    }),
  )
)
```

```js-eval
{
  "a": 2,
  "d": 3,
  "b": 2
}
```

#### Nested (+ -> {})

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in nested left (+ -> {})
output(
  add(
    create_obj({
      a: 1,
      d: 3
    }),
    create_obj(({self, root}) => {
      self.set("a", 2);
      self.set("b", create_obj(({self, root}) => {
        self.set("c", root.get("a"));
      })({root}))
    }),
  )
)
```

```js-eval
{
  "a": 2,
  "d": 3,
  "b": {
    "c": 2
  }
}
```

What if right side does not have a?


```js
const { create_obj, add, output } = require("./docs/obj");
// Root in nested left (+ -> {}) where right side doesnt have field
output(
  add(
    create_obj({
      a: 1,
      d: 3
    }),
    create_obj(({self, root}) => {
      self.set("b", create_obj(({self, root}) => {
        self.set("c", root.get("a"));
      })({root}))
    }),
  )
)
```

It takes the `a` from super side!

```js-eval
{
  "a": 1,
  "d": 3,
  "b": {
    "c": 1
  }
}
```

#### Nested ( {} -> + )

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in nested left ({} -> +)
output(
  create_obj(({self, root}) => {
    self.set("a", 1);
    self.set("b", add(
      create_obj({
        d: 3
      }),
      create_obj(({self, root}) => {
        self.set("a", 2);
        self.set("b", create_obj(({self, root}) => {
          self.set("c", root.get("a"));
        })({root}))
      })
    )({root}));

  })
)
```

Currently `root` points towards most upper `a` from right side.
The alternative is `origin`

```js-eval
{
  "a": 1,
  "b": {
    "d": 3,
    "a": 2,
    "b": {
      "c": 2
    }
  }
}
```

I think this is something I want to change...

## Origin

I want to introduce `origin` that would be like `root` but from `super`.

##### Root
Compare (`root`):

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in nested left ({} -> +)
output(
  create_obj(({self, root}) => {
    self.set("a", "the most top a");
    self.set("b", add(
      create_obj({
        d: "left side d"
      }),
      create_obj(({self, root}) => {
        self.set("a", "the most top a from right side");
        self.set("b", create_obj(({self, root}) => {
          self.set("a", "the most inner a from right side");
          self.set("c", root.get("a"));
        })({root}))
      })
    )({root}));

  })
)
```

```js-eval
{
  "a": "the most top a",
  "b": {
    "d": "left side d",
    "a": "the most top a from right side",
    "b": {
      "a": "the most inner a from right side",
      "c": "the most top a from right side"
    }
  }
}
```

### Super

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in nested left ({} -> +)
output(
  create_obj(({self, root}) => {
    self.set("a", "the most top a");
    self.set("b", add(
      create_obj({
        d: "left side d",
        a: "left side a",
      }),
      create_obj(({self, root, super_}) => {
        self.set("a", "the most top a from right side");
        self.set("b", create_obj(({self, super_}) => {
          self.set("a", "the most inner a from right side");
          self.set("c", super_.get("a"));
        })({root, super_}))
      })
    )({root}));

  })
)
```

```js-eval
{
  "a": "the most top a",
  "b": {
    "d": "left side d",
    "a": "the most top a from right side",
    "b": {
      "a": "the most inner a from right side",
      "c": "left side a"
    }
  }
}
```

### Self

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in nested left ({} -> +)
output(
  create_obj(({self, root}) => {
    self.set("a", "the most top a");
    self.set("b", add(
      create_obj({
        d: "left side d",
        a: "left side a",
      }),
      create_obj(({self, root, super_}) => {
        self.set("a", "the most top a from right side");
        self.set("b", create_obj(({self, super_}) => {
          self.set("a", "the most inner a from right side");
          self.set("c", self.get("a"));
        })({root, super_}))
      })
    )({root}));

  })
)
```

```js-eval
{
  "a": "the most top a",
  "b": {
    "d": "left side d",
    "a": "the most top a from right side",
    "b": {
      "a": "the most inner a from right side",
      "c": "the most inner a from right side"
    }
  }
}
```

### Origin

```js
const { create_obj, add, output } = require("./docs/obj");
// Root in nested left ({} -> +)
output(
  create_obj(({self, root}) => {
    self.set("a", "the most top a");
    self.set("b", add(
      create_obj({
        d: "left side d",
        a: "left side a",
      }),
      create_obj(({self, root, super_, origin}) => {
        self.set("a", "the most top a from right side");
        self.set("b", create_obj(({self, super_, origin}) => {
          self.set("a", "the most inner a from right side");
          self.set("c", origin.get("a"));
        })({root, super_, origin}))
      })
    )({root}));

  })
)
```

```js-eval
{
  "a": "the most top a",
  "b": {
    "d": "left side d",
    "a": "the most top a from right side",
    "b": {
      "a": "the most inner a from right side",
      "c": "the most top a"
    }
  }
}
```

