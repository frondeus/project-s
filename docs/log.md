# What I want

I want to create simple LISP-like language.
It should have a good LSP support and static typing (HM-inference).

It should be created incrementally, starting with just numbers.

I want to expand it horizontally not vertically, so in the beginning the language would support only numbers (evaluating constants) but already have working LSP, REPL, evaluation and acceptance tests.


# Long term goal

I want to have something similar in functionality to Jsonnet (https://jsonnet.org) especially i like adding two structs together. That is brilliant and exactly what i want from my lang.


# Log 14.05
Added tree-sitter-s. It's parsing CST. Added `parser.rs` - its converting CST into AST.

# Log 19.05
Added integration tests in markdown.
Lets check it:

```
(1 2 3)
```

```cst
List([Number(1.0), Number(2.0), Number(3.0)])
```

Cool!.

XTask was also added. Now we can start writing the language itself!

Okay, so what I want?
I want to at least have

```
(struct (quote (
    :name "Name"
    :surname "Surname"
)))
```

and have it compiled to 

```json
{
  "name": "Name",
  "surname": "Surname"
}
```

Let's write the execution of such

# Log 20.05

Okay. That works.
But clearly i will not trust Cursor for long. Let's do it properly, manually and with the right approach.

Let's start with a number:

```
5
```

```json
5.0
```

# Log 21.05

Okay, now a string. I will definitely need a string. 
For now simple string will be enough. No escaping, nothing that fancy

```
"5"
```

```json
"5"
```

Cool. Before I go next. I think its time to extend the language vertically.
First - static typing

But before we do that, maybe its time to express our tree as a flat array 
Now that we have it, we can apply some basic typechecking

```
5
```

```type
Number
```

```
"41"
```

```type
String
```

Cool. Now the next thing should be type ascription in the expression

```
(is-type 5 Number)
```

```json
true
```

```
(is-type 5 String)
```

```json
false
```

```
(is-type "5" Number)
```

```json
false
```

```
(is-type "5" String)
```

```json
true
```

# 24.05

Cool. It's not finished but at least we started doing something.
If it's lisp it should allow quoting:

```
(quote 1)
```

Primitive atoms are evaluating to themselves

```json
1.0
```

```
(quote "42")
```

```json
"42"
```

Symbols are evaluating to themselves as well

```
(quote quote)
```

```json
"quote"
```

Any advanced sexp is just kept as is.

```
(quote (1 2 3))
```

Printing to JSON is wrapping sexp in Strings
```json
"(1 2 3)"
```

Now we can use this quote to actually properly construct struct

```
(struct (quote (
    :name "Name"
    :surname "Surname"
)))
```

and have it compiled to 

```json
{
  "name": "Name",
  "surname": "Surname"
}
```

For now we treat as if struct was a function that took SExpression.
Later when we introduce macros, we might want to change that.

For now, we use full `quote`. But it makes sense to expand the grammar to introduce `'`.

```
(struct '(
    :name "Name"
    :surname "Surname"
))
```

and have it compiled to 

```json
{
  "name": "Name",
  "surname": "Surname"
}
```

Ok. Cool, this is nice and shit, but this language is useless if we cannot do any operation.
Let's for now add hardcoded add operator

```
(+ 1 2)
```

```json
3.0
```

Now we need a way to calculate that op as a field of struct.
Which means quasiquote and unquote operators.

This is using quote:
```
'(1 2 (+ 1 2))
```

```json
"(1 2 (+ 1 2))"
```

But this is using quasiquote

```
(quasiquote (1 2 (+ 1 2)))
```

Still no difference:

```json
"(1 2 (+ 1 2))"
```

Until we use unquote

```
(quasiquote (1 2 (unquote (+ 1 2))))
```

```json
"(1 2 3)"
```


However in order to make it work, i need to first:
* Make sure we can create new AST on the fly.
* Make sure that when we are accessing SExp, we are asking the right AST tree.

The easiest solution is to keep reference to AST inside of SExp.
But... where?

Or maybe not. Lifetimes will eat me alive.
Maybe i just need to clone the whole quasiquoted tree.

Yep. For now cloning will work.

Ok, now reader shortcut.

```
`(1 2 ,(+ 1 2))
```

```json
"(1 2 3)"
```

Cool. Now can we use it in our struct definition?

```
(struct `(
  :name "John Smith"
  :age ,(+ 20 3)
))
```

```json
{
  "age": 23.0,
  "name": "John Smith"
}
```

NICE.

Okay. What's next?
It would be nice to be able to add two
objects together.
However, I think first i need to address lack of variables.

```
(let x 2 (+ x x))
```

```json
4.0
```

So now just using symbol should return error
```
x
```

```json
"<Error: Undefined variable: x>"
```

Okay, just like jsonnet i want to be able to create new variables inside of structs.

```
`(:key (+ 1 2))
```

```json
"(:key (+ 1 2))"
```


Apparently we have bug in quasiquoting.
Which isn't surprising.
The problem is:

```
(struct `(
  :key '(+ 1 2)
))
```

```json
{
  "key": "(+ 1 2)"
}
```


The quasiquote returns SExpression that is generation further
than the parent.

How? Currently quasiquoting is creating new AST.
That AST contains (+ 1 2) expression.
But later, We keep it as a reference and when we try to print that expression,
the AST is lost. So we have a dangling pointer of sorts.

Ok fixed.
The only downside is - it's going to leak memory over time.
But we need GC anyway...

Ok, going back to let expressions

```
(struct `(
  (let x 5)
  :key (+ 1 x)
  :another '(+ 1 x)
))
```

```json
{
  "another": "(+ 1 x)",
  "key": 6.0
}
```

Oooh, I get why this is not working.

The problem is, that unquoting happens BEFORE we start digesting struct.
Which fucking make sense. That is intended behaviour.
We do quasiquote and only AFTER that we pass that to the struct function
in order to generate struct from SExp.

The only thing i can do is to reverse the inner quoting.
Bingo.

Ok, what about "self"?

There are two options i see.
Either I choose lazy evaluation, or I need to somehow sort the field initialization by the DAG. Assuming there is a DAG.

For example if we have:

```example
(struct `(
  :key (+ 1 self.another)
  :another (+ 1 1)
))
```

then we need to reorder the struct into

```example
(struct `(
  :another (+1 1)
  :key (+ 1 self.another)
))
```
because then `:another` key exists in env.

Another point is - in order to use `self` we need struct accessing
first.

As in accessing field of the struct

The simplest way is to have it like this:

```
( 
  (struct '(:key 1 :another 2))
  :another
)
```

```json
2.0
```

In other words, structure is used as if it was a function and first
parameter tells you the key accessed

It works even better if struct is named

```
(let foo (struct '(:key 1 :another 2))
  (foo :another)
)
```

```json
2.0
```

Cool. So, `self`...

Let's say that for now keys HAVE TO
be ordered explicitly

```
(struct `(
  :another (+ 1 1)
  :key (+ 1 (self :another))
))
```

```json
{
  "another": 2.0,
  "key": 3.0
}
```

Okay, but how do i access object? It's not in the env. And it cannot be yet
in some Arc<_> because it is still being mutated.

Ok, now next one has to fail, right?

```
(struct `(
  :key (+ 1 (self :another))
  :another (+ 1 1)
))
```

```json
{
  "another": 2.0,
  "key": "<Error: Undefined key: another>"
}
```

Yep.

Okay, now the only thing missing is "root".

```
(struct `(
  :another 4
  :key (struct '(
    :a 1
    :b (+ 1 (root :another))
  ))
))
```

```json
{
  "another": 4.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  }
}
```

Damn, that's cool!

Does it work with accessing self?

```
(struct `(
  :another 4
  :key (struct '(
    :a 1
    :b (+ 1 ((root :key) :a))
  ))
))
```

```json
{
  "another": 4.0,
  "key": {
    "a": 1.0,
    "b": "<Error: Undefined key: key>"
  }
}
```

No. It doesnt because key was not created yet. Makes sense.

Ok, now i want to have another reader shortcut

```
{
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}
```

```json
{
  "another": 4.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  }
}
```

# 25.05

Okay. What else am I missing?

I guess one reason I started this project is to be able to add two structs
together like in the JSONnet.

So... Let's maybe focus on it.

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

{
  :another 9
})
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  }
}
```

Oooh... Okay. I see.
It is returning `b` = `5.0` because overriding `another` doesn't really
change `b`. Because `b` was already calculated in the left side.
And since `b` is not lazy... 

So I expected a 10.0 but that is impossible with the current lang.
Damn. That's why jsonnet is a lazy language, right?

Okay let's not focus right now on it. Maybe it won't be such an issue.
I want to add `super`!

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

(thunk () {
  :another 9
  :self (+ 1 (self :another))
  :super (+ 1 (super :another))
}))
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0
  },
  "self": 10.0,
  "super": 5.0
}
```

For now we are cloning supers over and over. That should
change but I don't have time or energy to do it now.
Maybe what we need is some kind of reference value in the future.

Okay, what if I override nested struct?

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

{
  :another 9
  :key 10
})
```

```json
{
  "another": 9.0,
  "key": 10.0
}
```

Makes sense.

Okay. Next thing is - If i want to have a partial adding of the struct,
I need to have conditionals on the keys.

That means, I need proper booleans.


```
true
```

```json
true
```

```
false
```

```json
false
```

Ok now the conditionals inside of objects

```
{
  (if true '(:key 42))
  (if false '(:false 13))
  (if false '(:true 10) '(:else 12))
}
```

```json
{
  "else": 12.0,
  "key": 42.0
}
```

Okay, now the only missing element is `has?` operator.

```
(let x {
  :key 42.0
}
  (has? x :key)
)
```

```json
true
```

```
(let x {
  :key 42.0
}
  (has? x :another)
)
```

```json
false
```

Okay, now we should be able to emulate jsonnet behaviour:

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

(thunk () {
  :another 9
  
  (if (has? super :key)
    '(:key 
      (+ (super :key) {
        :c 3
      })
    )
    '(:key {:c 3})
  )

}))
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0,
    "c": 3.0
  }
}
```

Yay! It works. Now what I need is some kind of macro to hide that ugliness

Question - how is the "super" behaving. Is it more like a `root` but for previous struct?
or `self`?

```
(+ {
  (let x 4)
  :another x
  :key {
    :a 1
    :b (+ 1 (root :another))
  }
}

(thunk () {
  :another 9
  
  (if (has? super :key)
    '(:key 
      (+ (super :key) (thunk () {
        :c (+ (super :a) 3)
      }))
    )
    '(:key {:c 3})
  )

}))
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0,
    "c": 4.0
  }
}
```

It.. works as intended! Yay

Ok. Does it work if the left side is in a variable?

```
(let left {
    (let x 4)
    :another x
    :key {
      :a 1
      :b (+ 1 (root :another))
    }
  }

(+ left (thunk () {
  :another 9
  
  (if (has? super :key)
    '(:key 
      (+ (super :key) {
        :c 3
      })
    )
    '(:key {:c 3})
  )

}))

)
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0,
    "c": 3.0
  }
}
```

Yes.

What about right side?

```
(let left {
    (let x 4)
    :another x
    :key {
      :a 1
      :b (+ 1 (root :another))
    }
  }

(let right {
  :another 9
  
  (if (has? super :key)
    '(:key 
      (+ (super :key) {
        :c 3
      })
    )
    '(:key {:c 3})
  )

}

(+ left right)

))
```

```json
"<Error: super used outside of object>"
```

Yeah...
That makes sense since we are eager :)

What if in order to make it lazy, i would pass a quoted struct?
(EDIT from 01.06: changed it to use thunks.).

```
(let left {
    (let x 4)
    :another x
    :key {
      :a 1
      :b (+ 1 (root :another))
    }
  }

(let right (thunk () {
  :another 9
  
  (if (has? super :key)
    '(:key 
      (+ (super :key) {
        :c 3
      })
    )
    '(:key {:c 3})
  )

})

(+ left right)

))
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0,
    "c": 3.0
  }
}
```

Macros:

So, I guess the next step would be to have a macro


```
(
  (macro (x y) `(+ ,x ,y))
  1 2
)
```

```json
3.0
```

Ok, does it work?

```
(
  let var (macro (name value body) `(let ,name ,value ,body))
  (var x 4.2 x)
)
```

```json
4.2
```

The problem is, we are evaluating arguments which is counterpoint of macros.

Ok. Fixed.

Ok, out of curiosity. Will that work :D

```
(
  let macrodef (macro (name args body in) `(let ,name (macro ,args ,body) ,in))
  (macrodef var (name value body) `(let ,name ,value ,body)
    (var x 4.2 x)
  )
)
```

```json
4.2
```

# 26.05

Ok, the next thing would be to be able to call macro from inside of an object
For now let's make it work so that only one command
can be generated by the macro call.


```
(
  let fif (macro (name co the els) `(if ,co '(,name ,the) '(,name ,els)))
  {
    (fif :key true 42.0 10.0)
  }
)
```

```json
{
  "key": 42.0
}
```

Great!
Ok, now we should be able to clean the object adding.

```
(let add-obj (macro (key value) 
  `(if (has? super ,key)
    '(,key (+ (super ,key) ,value))
    '(,key ,value)
  )
)
(let left {
    (let x 4)
    :another x
    :key {
      :a 1
      :b (+ 1 (root :another))
    }
  }

(let right (thunk () {
  :another 9
  
  (add-obj :key {:c 3})
})

(+ left right)

)))
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0,
    "c": 3.0
  }
}
```

NICE

What's else missing?
* [ ] Loops
* [x] Functions
  * [ ] Closures!
  * [x] Rust functions
  * [ ] Rust macros?
* [ ] Imports
* [ ] If else in normal places

Nice to have
* [ ] Destructuring
* [ ] Arrays
* [ ] Better strings

Probably a lot of more lol.
Like vertical stack
* [ ] LSP
* [ ] Type checker!

Let's focus on Functions

```
((fn (x y) (+ x y)) 1 2)
```

```json
3.0
```

# 27.05

Ok lets see if native functions also work.
I added prelude with `-`.

```
(- 3 2)
```

```json
1.0
```

Great! Now I can move some of the code to functions.

Unfortunately we cannot use `+` yet because we are too
eager to eval arguments before setting up `super`.

Can i make it work?
Either:
* Use native macro instead
* Or maybe even better, pass iterator, not a vector?
  * Ah, shit, we cannot pass iterator because iterator would capture runtime that we already pass to the body...
  * Native macro also wont work because macro returns SExpression
  So we would need to serialize object to SExpression which might not be possible

  So it seems that for now the only solution would be to:
  * Keep it as a special form :vomit:
  * Make arguments lazy...


  It seems that arguments about making the language lazy are coming back like a boomerang.

# 30.05

It seems that in order to make it work i need to create Thunks.
And Thunks are in my opinion Closures with extra step of caching its value.
So in the end my next step is to add closures.

How to do that?
When declaring function i need to have a separate pass that detects function declaration
Then it should find all free variables.
Then it should add an extra parameter to the function which would be a struct of free variables.
And then modify the body by saying that it uses fields from that closure struct.


Ok so ... 

```
(let c 42
(fn (a b) (a b c)))
```

Question is that rose during implementation - do i really want to have it as a separate pass?
Or just during the evaluation...

I guess separate pass would make sense but then i need to keep track which root id is root id and not assume that the one from parsing is still one.

The problem is, with that kind of lambda lifting we are kinda loosing
information about captured context.


```lift
(let c 42 (cl (a b) (c) (a b ($$closure :c))))
```

Ok, i lifted the function into "closure declaration" that takes another list outside of
signature which is explicit list of free variables. Also accessing free variable is now
with `(_closure c)`.


Also additional "problem" is, currently lambda lifting is very naive.
If we have undefined variable, then it will be assumed to be captured!


But we can handle it some other time lmao.
Anyway, with that lifting I guess - we should have handling in runtime for "cl" :D

Anyway x2

There is also a problem of quoting

```
(fn (a b) '(+ a b c))
```

```lift
(fn (a b) (quote (+ a b c)))
```

What about quasiquoting?

```
(fn (a b) `(+ a b c))
```

```lift
(fn (a b) (quasiquote (+ a b c)))
```

Better. But unquote wont work...

```
(let d 42.0 
(fn (a b) `(+ ,a ,b c ,d))
)
```

```lift
(let d 42 (cl (a b) (d) (quasiquote (+ (unquote a) (unquote b) c (unquote ($$closure :d))))))
```

Yep, that's correct.
Coolio.

# 31.05

Ok, now having that "lift" (is it lift tho?)
i can modify the runtime.


```
(let top (
  fn () (
    let c 42.0
    (fn (a b) (+ a b c))
  )
)

((top) 1.0 2.0)
)
```

```lift
(let top (fn () (let c 42 (cl (a b) (c) (+ a b ($$closure :c))))) ((top) 1 2))
```

```json
45.0
```

Bingo!

Okay, but is it fully complete?

```
(let c 42.0
  (fn () (
    let d 10.0
    (+ c d)
  ))
)
```

```lift
(let c 42 (cl () (c) (let d 10 (+ ($$closure :c) d))))
```

Yeah it doesnt look good.

>Also additional "problem" is, currently lambda lifting is very naive.
>If we have undefined variable, then it will be assumed to be captured!

That is a cultprit. I guess we need to emulate lexical scoping in that pass.

Ok. Fixed!

One thing tho. I fixed normal `let`.
What about let in objects?

```
(({
  :a 42.0
  :b (fn () (self :a))
} :b))
```

```lift
(((struct (quote (:a 42 :b (cl () (self) (($$closure :self) :a))))) :b))
```

```json
42.0
```

Okay, self works. Now root!

```
((({
  :a 42.0
  :b {
    :c (fn () (root :a))
  }
} :b) :c))
```

```lift
((((struct (quote (:a 42 :b (struct (quote (:c (cl () (root) (($$closure :root) :a)))))))) :b) :c))
```

```json
42.0
```

And now finally super

```
(( (+ {
  :a 42.0
}

(thunk () {
  :a (fn () (+ (super :a) 10.0))
})) :a))
```

```json
52.0
```

Ok, good. Now let's clean this code a little bit!

So now, we got closures.

What's else missing?
* [ ] Loops
* [/] Thunks
* [x] Functions
  * [x] Closures!
  * [x] Rust functions
  * [x] Rust macros?
* [ ] Imports
* [ ] If else in normal places

Nice to have
* [ ] Destructuring
* [ ] Arrays
* [ ] Better strings

Probably a lot of more lol.
Like vertical stack
* [ ] LSP
* [ ] Type checker!

A little reminder.
Next step is a Thunk.
Basically how i see it, a Thunk is a Closure with extra steps and differences
First of all, it:
* Has no signature, just captured variables
* It has some internal state that is initiated during first evaluation that replaces itself. A.K.A. Cache.

But intuition tells me, i dont want to have a Thunk EVERY time.
I should be able to detect when a thunk is needed or not.
Like, only if a function is called? Or when `super` `root` and `self` is used?

I think i can split it into two stages.
First, introduce `(thunk expression)` and make sure runtime supports it fully.
Then figure out where to put it automatically with a separate pass.

Also, one thing i need to remember - When evaluating the quoted content i need to first run lambda lifting pass.
Or, all passes AFTER the AST construction.

Okay, first things first!

```
(thunk () 123)
```

First argument is list of captured variables
Next is the body

```json
"<Thunk: Thunk { inner: RefCell { value: ToEvaluate { captured: {}, body: SExpId { id: 3, generation: 0 } } } }>"
```

Okay.

```
(let x 42.0 
  (thunk (x) (+ 123 x))
)
```

```json
"<Thunk: Thunk { inner: RefCell { value: ToEvaluate { captured: {\"x\": Number(42.0)}, body: SExpId { id: 8, generation: 0 } } } }>"
```

Cool.
Now what if we use it?

```
(let x (thunk () 42.0)
  (+ x 1.0)
)
```

```json
43.0
```

Ok...
Kinda works.
But for now it works as a `getter` kind of function.
Because it does not cache the result.

Okay. Still works, but im not sure if it really calculates once or twice.

```
(
  let x (thunk () (print 42.0))
    (+ x x)
)

```

```log
Number(42.0)

```

```json
2.0
```

Only once! Wooho!

# 01.06

Okay so now we have thunks but we are not automatically using them (yet).

One thing we tried to emulate before thunks was adding objects.
We used quoting for that but it felt janky. Now we have a better way:

```
(let left {
    (let x 4)
    :another x
    :key {
      :a 1
      :b (+ 1 (root :another))
    }
  }

(let right {
  :another 9
  
  (if (has? super :key)
    '(:key 
      (+ (super :key) {
        :c 3
      })
    )
    '(:key {:c 3})
  )

}

(+ left right)

))
```

```json
"<Error: super used outside of object>"
```

But now, we should be able to make a right side a thunk!
In the future, we will be able to do it automatically, but for now:

```
(let left {
    (let x 4)
    :another x
    :key {
      :a 1
      :b (+ 1 (root :another))
    }
  }

(let right (thunk () {
  :another 9
  
  (if (has? super :key)
    '(:key 
      (+ (super :key) {
        :c 3
      })
    )
    '(:key {:c 3})
  )

})

(+ left right)

))
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0,
    "c": 3.0
  }
}
```

Perfect!

So I guess now the only thing left is to automatically
inject thunks when necessary.

What are the cases?
* Use of `self`
* Use of `super` outside of `+` operator.
* Making `+` an STD function instead of special form. [x]

Can i do the latter?

Okay so I see the problem.
If we want `+` to be an STD function then every use of
`super` MUST be wrapped in a thunk.

Yeah, okay but then the function works :)

Next thing is we can take my macro for easy adding nested structs and expose it in std as well.

```example
(let add-obj (macro (key value) 
  `(if (has? super ,key)
    '(,key (+ (super ,key) ,value))
    '(,key ,value)
  )
)
```

My biggest problem right now tho is that writing any AST construction
in Rust is PITA.

Ok. I think i made it work. But does it work?

```
(let left {
    (let x 4)
    :another x
    :key {
      :a 1
      :b (+ 1 (root :another))
    }
  }

(let right (thunk () {
  :another 9
  
  (+obj :key {:c 3})
})

(+ left right)

))
```

```json
{
  "another": 9.0,
  "key": {
    "a": 1.0,
    "b": 5.0,
    "c": 3.0
  }
}
```

Whoah!

Ok. So. Thunks.
I need to automatically insert thunks.

What are the cases?
* Use of `self` and `root`
* Use of `super` 
  - If there is super used, we want to make `(thunk () {})` of the whole expression
  Let's start with that.

But before we do it.
I think its time to introduce other `.md` files.
Going back through log.md is just cumbersome at that point
