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

(debug "Before eager" result)
(debug "Eager result" (deep-eager result))
(debug "After eager" result)
 result
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

```traces
 INFO project_s::runtime::s_std: Debug: [
    String(
        "Before eager",
    ),
    Ref(
        RefCell {
            value: Object(
                {
                    "ancestry": Thunk(
                        Thunk/ToEvaluate,
                    ),
                    "features": Thunk(
                        Thunk/ToEvaluate,
                    ),
                    "languages": Thunk(
                        Thunk/ToEvaluate,
                    ),
                    "stats": Thunk(
                        Thunk/ToEvaluate,
                    ),
                },
            ),
        },
    ),
]
 WARN project_s::runtime::s_std: Deep eager: Object({"ancestry": Thunk(Thunk/ToEvaluate), "features": Thunk(Thunk/ToEvaluate), "languages": Thunk(Thunk/ToEvaluate), "stats": Thunk(Thunk/ToEvaluate)})
 WARN project_s::runtime::s_std: Deep eager: String("Dwarf")
 WARN project_s::runtime::s_std: Deep eager: Object({"stout": Thunk(Thunk/ToEvaluate)})
 INFO project_s::runtime::s_std: Setting
 INFO project_s::runtime::s_std: Setting RefCell { value: Object({"ancestry": Thunk(Thunk/Evaluated(String("Dwarf"))), "features": Thunk(Thunk/Evaluated(Ref(RefCell { value: Object({"stout": Thunk(Thunk/Evaluating)}) }))), "languages": Thunk(Thunk/ToEvaluate), "stats": Thunk(Thunk/ToEvaluate)}) } to Ref(RefCell { value: Object({"ancestry": Thunk(Thunk/Evaluated(String("Dwarf"))), "features": Thunk(Thunk/Evaluated(Ref(RefCell { value: Object({"stout": Thunk(Thunk/Evaluating)}) }))), "is_stout": Thunk(Thunk/ToEvaluate), "languages": Thunk(Thunk/ToEvaluate), "stats": Thunk(Thunk/ToEvaluate)}) })
 WARN project_s::runtime::s_std: Deep eager: String("Stout")
 WARN project_s::runtime::s_std: Deep eager: List([String("Common"), String("Dwarvish")])
 WARN project_s::runtime::s_std: Deep eager: Object({"cha": Thunk(Thunk/ToEvaluate), "con": Thunk(Thunk/ToEvaluate), "dex": Thunk(Thunk/ToEvaluate), "int": Thunk(Thunk/ToEvaluate), "str": Thunk(Thunk/ToEvaluate), "wis": Thunk(Thunk/ToEvaluate)})
 WARN project_s::runtime::s_std: Deep eager: Number(13.0)
 WARN project_s::runtime::s_std: Deep eager: Number(13.0)
 WARN project_s::runtime::s_std: Deep eager: Number(11.0)
 WARN project_s::runtime::s_std: Deep eager: Number(11.0)
 WARN project_s::runtime::s_std: Deep eager: Number(6.0)
 WARN project_s::runtime::s_std: Deep eager: Number(17.0)
 INFO project_s::runtime::s_std: Debug: [
    String(
        "Eager result",
    ),
    Ref(
        RefCell {
            value: Ref(
                RefCell {
                    value: Object(
                        {
                            "ancestry": Thunk(
                                Thunk/Evaluated(
                                    String(
                                        "Dwarf",
                                    ),
                                ),
                            ),
                            "features": Thunk(
                                Thunk/Evaluated(
                                    Ref(
                                        RefCell {
                                            value: Object(
                                                {
                                                    "stout": Thunk(
                                                        Thunk/Evaluated(
                                                            String(
                                                                "Stout",
                                                            ),
                                                        ),
                                                    ),
                                                },
                                            ),
                                        },
                                    ),
                                ),
                            ),
                            "is_stout": Thunk(
                                Thunk/ToEvaluate,
                            ),
                            "languages": Thunk(
                                Thunk/Evaluated(
                                    List(
                                        [
                                            String(
                                                "Common",
                                            ),
                                            String(
                                                "Dwarvish",
                                            ),
                                        ],
                                    ),
                                ),
                            ),
                            "stats": Thunk(
                                Thunk/Evaluated(
                                    Ref(
                                        RefCell {
                                            value: Object(
                                                {
                                                    "cha": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                13.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "con": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                13.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "dex": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                11.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "int": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                11.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "str": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                6.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "wis": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                17.0,
                                                            ),
                                                        ),
                                                    ),
                                                },
                                            ),
                                        },
                                    ),
                                ),
                            ),
                        },
                    ),
                },
            ),
        },
    ),
]
 INFO project_s::runtime::s_std: Debug: [
    String(
        "After eager",
    ),
    Ref(
        RefCell {
            value: Ref(
                RefCell {
                    value: Object(
                        {
                            "ancestry": Thunk(
                                Thunk/Evaluated(
                                    String(
                                        "Dwarf",
                                    ),
                                ),
                            ),
                            "features": Thunk(
                                Thunk/Evaluated(
                                    Ref(
                                        RefCell {
                                            value: Object(
                                                {
                                                    "stout": Thunk(
                                                        Thunk/Evaluated(
                                                            String(
                                                                "Stout",
                                                            ),
                                                        ),
                                                    ),
                                                },
                                            ),
                                        },
                                    ),
                                ),
                            ),
                            "is_stout": Thunk(
                                Thunk/ToEvaluate,
                            ),
                            "languages": Thunk(
                                Thunk/Evaluated(
                                    List(
                                        [
                                            String(
                                                "Common",
                                            ),
                                            String(
                                                "Dwarvish",
                                            ),
                                        ],
                                    ),
                                ),
                            ),
                            "stats": Thunk(
                                Thunk/Evaluated(
                                    Ref(
                                        RefCell {
                                            value: Object(
                                                {
                                                    "cha": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                13.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "con": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                13.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "dex": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                11.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "int": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                11.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "str": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                6.0,
                                                            ),
                                                        ),
                                                    ),
                                                    "wis": Thunk(
                                                        Thunk/Evaluated(
                                                            Number(
                                                                17.0,
                                                            ),
                                                        ),
                                                    ),
                                                },
                                            ),
                                        },
                                    ),
                                ),
                            ),
                        },
                    ),
                },
            ),
        },
    ),
]

```