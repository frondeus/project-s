(let :id (fn (:x) x))

(let :fix-super (fn (:f :super) (do
    (let* :result (f result super))
    result
)))
(let :fix-record #(:
    # (fn ( (fn ('a (record)) 'a) ) 'a)
(fn (:f) (do
    (let* :result (f result (obj/plain)))
    result
)))
#)

(let :extend (fn (:base :ext) (do
    (fix-super ext base)
)))

(let :to-record (fn (:plain)
    (fn (:self :super) plain)
))

(let :extend-fn (fn (:base :ext) (do
    (to-record (extend base ext))
)))


(let :dwarf (id (obj/record
    :ancestry "Dwarf"
    :languages [ "Common" "Dwarvish" ]
    :features (obj/plain
        :stout "Stout"
    )
    :hp (+ (super :hp) 2) # Stout feature
)))

(let :fighter (fn (:self :super) (do
    (obj/extend super
        :weapons-proficiency [ "All weapons" ]
        :armor-proficiency [ "All armor" "All shields" ]
        :hp (+ (super :hp)
            (* (super :lvl) (roll "1d8"))
        )
        :features (obj/extend
            (super :features)
            :hauler "Hauler"

        )

    )
)))
#(let :fighter (obj/record
# ))

(let :base (obj/plain
    :hp 0
    :lvl 1
))

(let :stats  (obj/record
    :stats (fix-record (obj/record
        :str 6
        :dex 11
        :con 13
        :int 11
        :wis 7 # (thunk () (self :wis))
        :cha 13
    ))
))

# (let :result (extend! base stats dwarf fighter))

(let :before-fighter
    (extend
        (extend
            base
            stats
        )
        dwarf
    )
)

(let :result (extend
    before-fighter
    fighter
))

result
