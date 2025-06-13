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

# In order to evaluate all fields before priting the JSON we use `deep-eager` here.
(deep-eager result)
