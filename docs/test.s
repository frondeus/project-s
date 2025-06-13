(let :dwarf {
    :ancestry "Dwarf"
    :languages [ "Common" "Dwarvish" ]
    :features {
        :stout (do 
            # "Cursed" operation - when its evaluated (and it is lazy)
            # It will mutate the `origin` reference.
            (set origin (obj/new (+ origin {
                :is_stout true
            })))
            "Stout"
        )
        :tout '(:a 2 3)
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

# In order to evaluate all fields before priting the JSON we use `deep-eager` here.
(deep-eager result)
(doo)



