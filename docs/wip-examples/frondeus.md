```s
(let :new (fn (:base) (do
    (let* :result (base result {}))
    result
)))
(let :extend (fn (:base :ext) (do
    (let* :super (base super {})
          :result (ext result super)
    )
    result
)))


(let :dwarf (fn (:self :super) {
    :ancestry "Dwarf"
    :languages [ "Common" "Dwarvish" ]
    :features {
        :stout "Stout"
    }
    ..super
}))

(let :stats (fn (:self :super) {
    :stats (new (fn (:self :super) {
        :str 6
        :dex 11
        :con 13
        :int 11
        :wis (thunk () (self :str))
        :cha 13
    }))
    ..super
}))

(extend stats dwarf)
```

```eval auto-approve
val new : forall (('a, {}) -?-> 'a) → 'b = "<Function: LispFn>"
val extend : forall (('a, {}) -?-> 'a, ('b, 'c) -?-> 'b) → 'd = "<Function: LispFn>"
val dwarf : forall ('a, 'b) → 'b extends {ancestry: "Dwarf", languages: ["Common" ∨ "Dwarvish"], features: {stout: "Stout"}} = "<Function: LispFn>"
val stats : forall ('a, 'b) → 'b extends {stats: {str: 6, dex: 11, con: 13, int: 11, wis: 6, cha: 13}} = "<Function: LispFn>"
- : {stats: {str: 6, dex: 11, con: 13, int: 11, wis: 6, cha: 13}, ancestry: "Dwarf", languages: ["Common" ∨ "Dwarvish"], features: {stout: "Stout"}} = {
  "ancestry": "Dwarf",
  "features": {
    "stout": "Stout"
  },
  "languages": [
    "Common",
    "Dwarvish"
  ],
  "stats": {
    "cha": 13.0,
    "con": 13.0,
    "dex": 11.0,
    "int": 11.0,
    "str": 6.0,
    "wis": 6.0
  }
}
```
