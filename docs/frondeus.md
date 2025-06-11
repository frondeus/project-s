```
(let :dwarf {
    :ancestry "Dwarf"
    :languages [ "Common" "Dwarvish" ]
    :features {
        :stout (do 
            (set origin (obj/new (+ origin {
                :is_stout true
            })))
            "Stout"
        )
    }
})

# Usually creating a new object or adding two objects is extra lazy -
# It creates a object **constructor** that is called only on the first access.
# Also its not cached, so if you call `result` twice, it creates two instances of the object.
# But here, we want to have an object reference so we call `obj/new` explicitly.

(let :result (obj/new (+ {
    :stats {
        :str 6
        :dex 11
        :con 13
        :int 11
        :wis 17
        :cha 13
    }
} dwarf)))

(deep-eager result)
```



```json
{
  "ancestry": "Dwarf",
  "features": {
    "stout": "Stout"
  },
  "is_stout": true,
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
    "wis": 17.0
  }
}
```
