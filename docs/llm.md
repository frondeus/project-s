# Context provided by Human

## EXECUTOR (compact)
- Auto-typed infra: TypeOf, FnSignature; Param<ID, T=Value>, Fun<(Args), Ret>; TypeOf for Vec<T>, Rest<T>, (T,), (A, B), Option<T>, Result<T, String>.
- Runtime bridges: FromValue/IntoValue for Option<T>, Result<T, String>, Param, (T,), Fun.
- Removed Fn1 and OptionV; inlined and auto-typed list/enumerate, list/map, list/find using Fun and Param; list/find now returns Result<Option<Param<1>>, String>.
- Prelude: with_auto_typed_fn_poly for list/*; with_auto_typed_fn_mono for >, <=, roll; manual types retained for +, -, *, =, ref/get, print, debug, tuple, list.
- All checks pass (format, clippy, parser, tests, snapshots).

## PLAN (towards full auto-typed)
- Short-term migrations:
  - "=": add auto-typed wrapper eq_typed(a: Param<1>, b: Param<2>) -> bool; register via with_auto_typed_fn_poly.
  - print/debug: add single-arg typed wrappers (Param<1> -> number / Param<1> -> Param<1>) and register auto-typed; keep existing variadic behavior functions.
  - get/ref: add RefOf<A>(Ref, PhantomData<A>) with TypeOf -> reference(A) and FromValue/IntoValue; register auto-typed wrappers.
- Medium-term:
  - Consider typed wrappers for list/tuple consistent with current prelude schemes, or align runtime semantics with declared types.
  - Explore FnSignature support for pure Rest<T> to migrate + and * automatically.
- Cleanup:
  - Remove remaining with_mono_type/with_poly_type after migrating all.
  - Refresh docs if type displays change.
- DoD: zero with_mono_type/with_poly_type usages; all via with_auto_typed_fn_mono/poly.

1. src/runtime has everything that happens during the runtime

s_std.rs is the file where we can define builtin functions and macros that
are written in Rust but are available in s-lang.

2. Typechecker is defined in src/types

We are using algebraic subtyping global inference inspired by
https://infoscience.epfl.ch/entities/publication/106da598-3385-4029-892b-27ea85194046

It has src/types/prelude.rs where we define type definitions of functions
defined in src/runtime/s_std.rs.

src/api.rs has high level traits and type definitions that are glue between low level runtime::Value and runtime::Env and high-level types.

This API allows us to define function as `fn add(left: f64, right: f64) -> f64` and Rust compiler knows, that when doing `env.with_fn("add", add)` that we define a function that takes precisely two parameters and returns a number. Howerver, currently `with_fn` does not register its type. That type is registered manually in prelude.rs

If you have any questions about ARCHITECTURE of the project, do not hesistate to ask human for clarification. Better than spending hours trying to understand the code on your own.

---

## INVESTIGATOR

Findings about current state:

- Runtime
  - Builtins are registered in `src/runtime/s_std.rs` via `Env::with_fn(name, rust_fn)`. There are also macros, but for this task we can ignore them.
  - The `api` layer (`src/api.rs` and `src/api/macros.rs`) provides:
    - `IntoNativeFunction` and `NativeFunction` to bridge Rust functions to runtime `Value`.
    - Support for functions with optional `&mut Runtime` and optional trailing `Rest<T>`.
    - A family of wrappers used in signatures: `EagerRec<T, Marker>`, `Keyword`, `Ref`, etc. These affect runtime evaluation but currently have no direct mapping to type inference.

- Types
  - The type inferer has its own prelude in `src/types/prelude.rs`. It registers:
    - Type constructors: `Some`, `None`, `Option`.
    - Builtin function types for operators (e.g., `+`, `-`, `*`, `>`, `<=`, `=`) and utility functions (`print`, `debug`, `tuple`, `list`, `list/enumerate`, `list/map`, `list/find`, `get`).
  - This prelude is manually kept in sync with the runtime’s builtins.
  - The `TypeBuilder` utilities build types and also record textual form in a `SourceBuilder`. Primitives: `number`, `bool`, `string`, `keyword`. Variadic tuples exist as `InferedType::Tuple { rest }` but had no ergonomic builder.
  - Equality `=` is registered polymorphically as `('a, 'b) -> bool` (by design).

- Gap we want to close
  - Adding a Rust builtin requires also adding a matching type in `types/prelude.rs`. We want: define the Rust function once and have the type prelude be derived automatically (with manual overrides for special cases).

Notes from HUMAN review (key guidance):
- Start by removing dead code and cleaning `s_std/functions.rs` (some wrappers become unnecessary).
- Variadic tuples are supported and can be modeled; however, current docs/snapshots use `[number] -> number` for `+/*`.
- Keep `=` polymorphic across any two types (worst case false).
- Consider markers to thread polymorphic variables (link input and output types) and wrappers to express higher-order function arg types.

---

## EXECUTOR outcome (what is implemented now)

1) Runtime Env can carry typed builtin schemes
- Added `runtime::BuiltinTy` and extended `runtime::Env`:
  - `with_mono_type`, `with_poly_type`
  - `with_typed_fn_mono`, `with_typed_fn_poly`
  - `iter_typed`, `get_typed`
- Types can now be seeded from runtime env(s) using:
  - `TypeEnv::with_runtime_prelude_envs(sources, &envs)`
  - `TypeEnv::with_runtime_prelude(sources, &env)`, delegating to the slice version
- Existing snapshots preserved by emitting builtins in the original order.

2) Auto-typing traits (no proc-macros, just traits and macro_rules for boilerplate)
- `api/typing.rs`:
  - `TypeGen`: stable type var allocator for markers
  - `TypeOf<T>`: maps Rust types to TypeBuilders
    - Implemented for `f64`, `i32`, `bool`, `String`, `Keyword`
    - `EagerRec<T, _>` erases to `T`
    - `Param<const ID, T>`: threads the same type variable across positions
    - `Fn1<A, B>`: higher-order arg type, maps to `function((A,), B)`
  - `FnSignature<Ctx>`: maps any `F: Fn` signature to a function type builder
    - Implemented for arities up to 3:
      - NO (no runtime): `Ctx = (O, A, B, ...)`
      - RT (with runtime): `Ctx = (O, A, B, ..., WithRuntime)`
    - Varargs (`Rest<T>`) intentionally not covered to avoid overlapping impls; keep explicit type overrides for `+/*` for now.

3) Auto-typed registration helpers
- In `api.rs`:
  - `Env::with_auto_typed_fn_mono<F, Ctx>(name, f)`
  - `Env::with_auto_typed_fn_poly<F, Ctx>(name, f)`
  - Work with any `F: IntoOverloadedFunction<Ctx> + FnSignature<Ctx>` (Ctx mirrors existing `IntoNativeFunction` macros).

4) s_std prelude adoption
- Switched `>` and `<=` to auto-typed registration.
- Left `+/*` and other special cases with explicit types (to match docs/snapshots and semantics).
- Registered typed schemes for other builtins (tuple, list, print, debug, list/enumerate, list/map, list/find, get, ref) within runtime env so `with_runtime_prelude_envs` has full data.
- Cleaned up `s_std/functions.rs` (removed dead code like object-constructor composition and `list_find_or`).

5) Call sites
- Type checking now seeded from runtime env(s) everywhere:
  - `lib.rs`, `lsp.rs` use `with_runtime_prelude_envs(modules.sources_mut(), &envs)`
  - Avoids cloning envs

6) Snapshot stability
- All `cargo xtask llm` checks pass; no snapshot changes introduced.

---

## PLANNER (next steps)

- Migrate more builtins to auto-typed registration:
  - `list/enumerate`: use `Param<1, _>` to tie element type
  - `list/map`: use `Param<1, _>`, `Param<2, _>` with `Fn1` for the mapping function
  - `list/find`: similar to `list/map` but return `Option 'a`
- Keep overrides for:
  - `+/*` (list varargs)
  - `=` polymorphic across any two types
  - `ref/get` (ref-polymorphism)
- Optional future work:
  - Literal wrappers (e.g., `StrLit`, `KeywordLit`) if we want to expose literal types in API
  - Consider adding safe, non-overlapping patterns to support `Rest<T>` in FnSignature if needed

---

## REVIEWER notes

- The auto-typing layer is additive and non-invasive: existing manual prelude remains the source of truth for order and semantics; runtime env seeding matches it.
- We’ve proven the approach by auto-typing `>` and `<=` without changing snapshots.
- The extension is ready to cover more builtins as we proceed, while preserving explicit overrides for tricky cases.


---

## INVESTIGATOR

Findings about current state:

- Runtime
  - Builtins are registered in `src/runtime/s_std.rs` via `Env::with_fn(name, rust_fn)`. There are also macros, but for this task we can ignore them.
  - The `api` layer (`src/api.rs` and `src/api/macros.rs`) provides:
    - `IntoNativeFunction` and `NativeFunction` to bridge Rust functions to runtime `Value`.
    - Support for functions with optional `&mut Runtime` and optional trailing `Rest<T>`.
    - A family of wrappers used in signatures: `EagerRec<T, Marker>`, `CalledConstructor<T>`, `Keyword`, `Ref`, etc. These affect runtime evaluation but currently have no direct mapping to type inference.
    HUMAN comment: I think if you START with removing dead_code and clearing up s_std.rs you may notice that some of those wrappers are no longer necessary and therefore your analysis will change.

- Types
  - The type inferer has its own prelude in `src/types/prelude.rs`. It registers:
    - Type constructors: `Some`, `None`, `Option`.
    - Builtin function types for operators (e.g., `+`, `-`, `*`, `>`, `<=`, `=`) and utility functions (`print`, `debug`, `tuple`, `list`, `list/enumerate`, `list/map`, `list/find`, `get`).
  - This prelude is manually kept in sync with the runtime’s builtins.
  - The `TypeBuilder` utilities build types and also record textual form in a `SourceBuilder`. Primitives available: `number`, `bool`; there’s also `TypeEnv::STRING` primitive, but no helper `string()` builder yet.
  - Varargs in the type prelude are modeled in two ways:
    - True list argument, e.g. `+ : [number] -> number`, `* : [number] -> number`.
    - Fixed arity binary ops with `function((lhs, rhs), ret)`, e.g. `-`, `>`, `<=`.
    - Higher-order list functions take `Function` arguments with explicit function types in the type prelude (e.g., `function((a,), b)`).
    Note from HUMAN:
    Yes, but note that in types.rs InferedType::Tuple has rest field.
    So in fact our type system supports variadic tuples! It's just we never used this feature in prelude.rs so there is no builder written for it in builder.rs
  - Equality `=` is registered polymorphically as `('a, 'b) -> bool` (no constraints between `'a` and `'b` right now).
    Note from HUMAN: This is by design. Any two types can be always compared together (and in worst case return false)

- Gap we want to close
  - Today, adding a Rust builtin requires adding a matching type declaration in `types/prelude.rs`. This is error-prone and tedious.
  - We want: define the Rust function once and have the type prelude be derived automatically.

Constraints and opportunities:

- Assumptions from the hint:
  - No overloading (good; simplifies mapping).
  - Ignore macros.
  - We can clean up dead code in `s_std/functions.rs` later.
  Note from HUMAN: I'd strongly suggest starting with that, then re-investigate what's left, update the plan and continue.
- What can be inferred automatically from Rust signatures:
  - Simple ground types: `f64 -> number`, `i32 -> number`, `bool -> bool`, `String -> string`.
  - `Rest<T>` can map to `[T]` in simple “pure varargs” cases (no fixed prefix args).
  - `EagerRec<T, _>` and `CalledConstructor<T>` can erase to `T` at the type level.
  Note from HUMAN: Generous tip - CalledConstructor will go away with your cleanup.
  - `&mut Runtime` is erased from the type.
- What requires manual annotation:
  - Any parameter or return typed as runtime `Value` (erases type info).
  Note from HUMAN:
   Okay so as a user of the API i currently have to write for example
   ```rust
   pub fn list_enumerate(list: Vec<Value>) -> Vec<(i32, Value)> {
    list.into_iter()
        .enumerate()
        .map(|(index, value)| (index as i32, value))
        .collect()
   }
   ```
   In theory that means the type is ([any]) -> [(number; any)], right?
   However it would be better (and currenly manually hardcorded) as (['a]) -> [(number; 'a)].
   Therefore, I wish i could write something like
   ```rust
   pub fn list_enumerate(list: Vec<Var<ID=1>>) -> Vec<(i32, Var<ID=1>)> {...}
   ```
   Using some kind of const generics. If that is possible. Something that links that we still return the same type.


  - Function-typed parameters (`Function`) where argument/return types are not visible from Rust (e.g., `list/map`).
  Note from HUMAN:
  Could we define such a type that allows us to define its subtype? Like `Function<LHS, RHS>`?
  - References (`Ref`) need `ref 'a` in the type system; Rust runtime doesn’t encode the `'a` in `Ref`, so we need explicit type.
  - Mixed fixed-arity + `Rest<T>` signatures (e.g., `add_numbers(first, Rest)`) are better modeled as `[T] -> U`, which requires an override.
  Note from HUMAN:
  but if we endup with `(T, ..[T]) -> U` i wont be mad.

Preliminary design direction:

- Introduce a type derivation trait family in `api` to map Rust function signatures to a type scheme:
  - `TypeOf<T>`: maps Rust parameter/return types to a type builder for the type inferer.
    Note from HUMAN: Would that be a trait or a type?
    - Core impls: `f64`, `i32` -> number; `bool` -> bool; `String` -> string; wrappers erase to inner `T` (e.g., `EagerRec<T, _>`, `CalledConstructor<T>`); `Keyword` -> primitive `keyword`.
    NOTE from HUMAN:
    Remember that s-lang has the notion of type literals. Like in typescript you can say that function ONLY accepts "yes", instead of any string. It would be sooo cool (but probably impossible with current Rust type?) to support it in our high level api as well.
    - Special cases: `Rest<T>` -> `[TypeOf<T>]` in pure vararg functions.
    - For `Value`, `Ref`, and `Function`, prefer manual overrides.
  - `FnSignature<F>`: given a Rust function type `F` (mirroring `IntoNativeFunction` macro families), produce a function type builder:
    - Drop `&mut Runtime`.
    - Map parameters via `TypeOf<T>`.
    - Map return via `TypeOf<O>`.
    - If the last parameter is `Rest<T>` and there are no fixed prefix args, produce `[TypeOf<T>] -> TypeOf<O>`.
- Extend `runtime::Env` API with a “typed registration” that stores the derived (or overridden) type scheme alongside the runtime function.
- Extend `TypeEnv` prelude construction with a variant that consumes these stored schemes to populate the type environment.

Notes:

- We will still support explicit overrides for builtins whose Rust signatures don’t carry enough information (e.g., `list/map`, `list/find`, `ref`, `get`, `=`, vararg `+/*`).
- Add `string()` and `keyword()` TypeBuilder helpers for completeness, since primitives exist in `TypeEnv`.

## PLANNER

Goal: When you register a builtin function in `s_std::prelude()` using a Rust function, its type is automatically registered in the type prelude, with manual overrides where Rust types are not expressive enough.

Plan of work (incremental, low-risk):

1) TypeBuilder ergonomics
- Add `string()` and `keyword()` helpers in `src/types/builder.rs` that map to `TypeEnv::STRING` and `TypeEnv::KEYWORD`.

2) Derivation traits in `api`
- Add `TypeOf<T>` that returns a “type builder” for `T`.
  - Impl for: `f64`, `i32` (number), `bool` (bool), `String` (string), `Keyword` (keyword).
  - Wrappers: `EagerRec<T, _>`, `CalledConstructor<T>` -> delegate to `TypeOf<T>`.
  - `Rest<T>` will be handled in the function signature lifting logic (see below).
  - Keep `Value`, `Function`, `Ref` unimplemented for auto derivation (forces manual override).
- Add `FnSignature<F>` that mirrors the `IntoNativeFunction` macro families:
  - Cases: NO (no runtime, no rest), RT, RE (rest only), RTRE.
  - Drop the `&mut Runtime` argument.
  - If the function is a pure vararg (`Fn(Rest<T>) -> O` or `Fn(&mut Runtime, Rest<T>) -> O`), map to `function(list(TypeOf<T>), TypeOf<O>)`.
  - Otherwise, map to `function(tuple(...params...), TypeOf<O>)`.
- Provide a helper to build a textual representation via `SourceBuilder` (same as current `types/prelude.rs` does), or return `InferedTypeId` builders directly. Prefer returning builders to avoid parsing.

3) Env API to capture builtin type schemes
- Extend `runtime::Env` with:
  - `with_typed_fn(name, func)` that:
    - Registers the runtime function (existing behavior).
    - Derives the type scheme via `FnSignature<F>` and stores a builder or a textual scheme in an internal registry.
  - `with_type_scheme(name, scheme)` to allow manual overrides for special cases (e.g., `+`, `*`, `=`, `ref`, `get`, higher-order list functions).
  - Read APIs to iterate stored schemes.

4) Type prelude construction using runtime env
- Add `TypeEnv::with_runtime_prelude(self, sources, &runtime::Env) -> Self`:
  - Preserve existing builtins for type constructors: `Some`, `None`, `Option` via the current code (or move to shared registration).
  - For each stored scheme in `runtime::Env`, build and register it with `with_mono` or `with_poly` depending on whether it contains type variables (we can start with monomorphic by default and allow an explicit “forall” marker in overrides).
  - Maintain the existing textual capture via `SourceBuilder` for traceability.
- Update call sites (e.g., `lib.rs::process_with_typechk` and tests) to construct the runtime prelude once (`s_std::prelude()`) and pass it to both runtime and typing:
  - `let prelude_env = s_std::prelude();`
  - `let mut type_env = types::TypeEnv::new().with_runtime_prelude(modules.sources_mut(), &prelude_env);`
  - `runtime.with_env(prelude_env);`

5) Seed manual overrides in `s_std::prelude()` using `with_type_scheme`/`with_typed_fn`
- Operators:
  - `+ : [number] -> number`
  - `- : (number, number) -> number`
  - `* : [number] -> number`
  - `> : (number, number) -> bool`
  - `<= : (number, number) -> bool`
  - `= : forall 'a 'b. ('a, 'b) -> bool`
- Utilities:
  - `print : forall 'a. 'a -> number`
  - `debug : forall 'a. 'a -> 'a`
  - `tuple : forall 'a. 'a -> 'a`
  - `list : forall 'a. [ 'a ] -> [ 'a ]`
  - `list/enumerate : forall 'a. ([ 'a ]) -> [ (number, 'a) ]`
  - `list/map : forall 'a 'b. ([ 'a ], ('a) -> 'b) -> [ 'b ]`
  - `list/find : forall 'a. ([ 'a ], ('a) -> bool) -> Option 'a`
  - `get : forall 'a. (ref 'a) -> 'a`
  - `ref : forall 'a. 'a -> ref 'a` (manual override)
- Enums/constructors:
  - Keep `Some`, `None`, `Option` constructors as in current `types/prelude.rs`, or move them to a shared registration helper accessible from both runtime/types.

6) Validation with snapshots
- Add entries to `docs/llm.md` using `auto-approve` snapshots to:
  - Display the combined runtime/type environment values and types of builtins.
  - A few small `eval` examples using `+`, `list/map`, `list/find`, and `ref/get` to ensure types and runtime behavior align.

7) Review and harden
- Ensure the derivation traits cover the supported builtin spectrum.
- Document how to add a new builtin with automatic typing and how/when to provide a manual override.

Stop here for human review before EXECUTOR changes.

---

## INVESTIGATOR Addendum (addressing HUMAN comments)

- Start with cleanup
  - Agreed to begin by removing dead_code and cleaning up `s_std/functions.rs` (and `s_std.rs` registrations). This will likely make some wrappers redundant (e.g., `CalledConstructor`) and simplify both runtime and the typing story. After cleanup, re-investigate what wrappers are still necessary.

- Variadic tuples already supported
  - Acknowledged that `InferedType::Tuple` has a `rest` field. We’ll add a builder in `types::builder` to express variadic tuples directly (e.g., `(T1, T2, ..[Tr])`). For `+`/`*`, we can express as just `(..[number]) -> number` (variadic tuple form), which aligns with the type system’s capabilities. Using `[number] -> number` is also fine but the tuple-rest form matches the internal representation better.

- Equality by design
  - Keeping `= : ('a, 'b) -> bool` polymorphic across any two types (worst case returns false). No extra constraints necessary.

- Polymorphism linking across inputs/outputs
  - Desire: write `list_enumerate(list: Vec<'a>) -> Vec<(number, 'a)>` at the Rust level. Proposal: a marker wrapper `Param<const ID: usize, T>` to thread the same type variable through multiple positions. Example: `list_enumerate(list: Vec<Param<1, Value>>) -> Vec<(i32, Param<1, Value>)>` derives `(['a]) -> [(number, 'a)]`. Implementation: `FromValue` for `Param<ID, X>` delegates to `X`; the typing derivation reuses the same type variable for the same `ID`.

- Higher-order function parameters
  - Proposal: typed wrappers for function-typed arguments, e.g., `Fn1<A, B>` (and `Fn2<...>` as needed) to carry argument/return types for the inferer. Derivation maps `Fn1<A, B>` to `function((A,), B)` in the type system. `FromValue` delegates to runtime `Function`.

- Trait vs type for derivation
  - Use a trait: `TypeOf<T>` providing a method to produce a type builder for `T`. This lets us implement it for wrappers (`Param`, `Fn1`, literal wrappers) and primitives. It also allows using a shared type-var context so the same `Param<ID, _>` maps to the same `'a`.

- Type literals in API
  - We can expose literal types with zero-cost marker wrappers, e.g., `StrLit<&'static str>`, `KeywordLit<&'static str>`, `NumLit<const N: i64>`. Derivation maps them to literal types in the inferer; at runtime, `FromValue` could optionally check literal equality (initially we can keep it typing-only, and treat runtime as `Value`-accepting).

- Varargs mapping
  - Mixed prefix + rest: map to variadic tuple `(T1, T2, ..[Tr]) -> U` via the new builder. Pure vararg: either `(..[T]) -> U` or `([T]) -> U`; prefer variadic tuple for consistency with the inferer. If we end up with `(T, ..[T]) -> U` that’s acceptable.

- CalledConstructor
  - Expect to remove as part of the cleanup.

---

## REVISED PLANNER

1) Cleanup (first)
- Remove dead_code from `s_std/functions.rs`, prune unused utilities and wrappers.
- Reassess which wrappers (e.g., `CalledConstructor`) are still required and remove what’s unnecessary.

2) Type builder ergonomics
- Add builders to `types::builder`:
  - `string()` and `keyword()` for primitives that already exist in `TypeEnv`.
  - `tuple_with_rest(items, rest)` (or equivalent) to materialize `InferedType::Tuple { rest }`.

3) API typing markers
- Introduce marker wrappers in `api` for better type derivation:
  - `Param<const ID: usize, T>` to thread the same type variable across args/returns.
  - `Fn1<A, B>` (and possibly `Fn2<...>`) to encode higher-order argument types.
  - Literal wrappers: `StrLit<&'static str>`, `KeywordLit<&'static str>`, `NumLit<const N: i64>`.
- Implement `FromValue` for these wrappers as delegations over existing `FromValue` impls.

4) Derivation traits
- Define `TypeOf<T>` as a trait that returns a builder for `T`. Provide impls for:
  - Ground types: `f64`/`i32`→number, `bool`→bool, `String`→string, `Keyword`→keyword.
  - Wrappers: `Param<ID, T>`, `Fn1<A, B>`, literal wrappers; `EagerRec<T, _>` erases to `T`.
- Define `FnSignature<F>` to map Rust function types (mirroring existing `IntoNativeFunction` macro families) to function type builders:
  - Drop `&mut Runtime`.
  - Support NO, RT, RE (vararg), RTRE cases.
  - Map pure varargs to variadic tuple `(..[T]) -> O`; mixed prefix+rest to `(prefix..., ..[T]) -> O`.

5) Env typed registration
- Extend runtime `Env` with:
  - `with_typed_fn(name, func)` to register runtime function and auto-derive its type scheme via `FnSignature`.
  - `with_type_scheme(name, scheme)` or builder-style override for cases where Rust types are insufficient (e.g., polymorphic or higher-order).
- Internally store derived schemes/builders to be consumed by the type prelude.

6) Type prelude integration
- Add `TypeEnv::with_runtime_prelude(self, sources, &runtime::Env)`:
  - Seed builtins from the runtime env’s stored schemes/builders.
  - Keep registration of `Some`, `None`, `Option` as in current `types/prelude.rs` initially; can be unified later.

7) Builtin mappings (examples)
- Use auto-derivation where possible:
  - `> : (number, number) -> bool`, `<= : (number, number) -> bool`, `- : (number, number) -> number`, `error : (string) -> any` (typed as `string -> error` or `string -> any`; we can retain `number` return now if staying consistent with runtime).
- Use markers/overrides:
  - `+`, `*` as `(..[number]) -> number`.
  - `print : forall 'a. 'a -> number` (derive with `Param` or override).
  - `debug : forall 'a. 'a -> 'a` (derive with `Param`).
  - `tuple : forall 'a. 'a -> 'a` (derive with `Param`).
  - `list : forall 'a. [ 'a ] -> [ 'a ]` (override).
  - `list/enumerate : forall 'a. ([ 'a ]) -> [ (number, 'a) ]` (derive via `Param`).
  - `list/map : forall 'a 'b. ([ 'a ], ('a) -> 'b) -> [ 'b ]` (use `Fn1` and `Param`).
  - `list/find : forall 'a. ([ 'a ], ('a) -> bool) -> Option 'a` (use `Fn1` and `Param`).
  - `ref : forall 'a. 'a -> ref 'a`, `get : forall 'a. (ref 'a) -> 'a` (override).
  - `= : forall 'a 'b. ('a, 'b) -> bool` (override; by design).

8) Validation
- Add snapshot tests in `docs/llm.md` with `auto-approve` to verify:
  - Types of registered builtins (from runtime env ingestion).
  - Example programs using `+`, `list/map`, `list/find`, `ref/get` with inferred types.

9) Review/iterate
- Re-run INVESTIGATOR after cleanup to confirm which wrappers remain.
- Adjust derivation/markers based on what’s still necessary.
