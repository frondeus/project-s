(let :fix-super (fn (:f :super) (do
    (let* :result (f result super))
    result
)))
(let :fix-record (fn (:f) (do
    (let* :result (f result (obj/plain)))
    result
)))

(let :extend (fn (:base :ext) (do
    (fix-super ext (fix-super base (obj/plain)))
)))


(let :dwarf (obj/record
    :ancestry "Dwarf"
    :languages [ "Common" "Dwarvish" ]
    :features (obj/plain
        :stout "Stout"
    )
))

(extend (obj/record
    :stats (fix-record (obj/record
        :str 6
        :dex 11
        :con 13
        :int 11
        :wis (thunk () (self :str))
        :cha 13
    ))
) dwarf)
