```
(let :dwarf {
    :ancestry "Dwarf"
    :languages [ "Common" "Dwarvish" ]
    :features {
        :stout (do 
            (set root (+ {} {
                :is_stout true
            }))
        )
    }
})

(+ {
    :stats {
        :str 6
        :dex 11
        :con 13
        :int 11
        :wis 17
        :cha 13
    }
} dwarf)
```

```json
{
  "ancestry": "Dwarf",
  "features": {
    "stout": "<Error: Expected symbol or string>"
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
    "wis": 17.0
  }
}
```