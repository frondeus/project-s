
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

(let :mod (fn (:stat) (do
    (let :stats [ 3 5 7 9 11 13 15 17 ])
    (let (:result :val) (list/find-or
        (list/map
            (list/enumerate stats)
            (fn ((:idx :val)) (tuple (- idx 4) val))
        )
        (fn ((:idx :val)) (<= stat val))
        (tuple 4 18)
        )
    )
    result
)))

(let :base (obj/plain
    :hp 0
    :lvl 1
))

(let :dwarf (obj/record
    :ancestry "Dwarf"
    :languages [ "Common" "Dwarvish" ]
    :features (obj/plain
        :stout "Stout"
    )
    :hp (+ (super :hp) 2) # Stout feature
))

(let :fighter  (obj/record
    :weapons-proficiency [ "All weapons" ]
    :armor-proficiency [ "All armor" "All shields" ]
    :hp (+ (super :hp)
        (* (super :lvl) (roll "1d8"))
    )
    :features (obj/extend
        (super :features)
        :hauler "Hauler"
    )
))


(let :result (extend (extend (extend
    base (obj/record
        :stats (fix-record (obj/record
            :str 6
            :dex 11
            :con 13
            :int 11
            :wis 7
            :cha 13
            :str-mod (thunk () (mod (self :str)))
            :dex-mod (thunk () (mod (self :dex)))
            :con-mod (thunk () (mod (self :con)))
            :int-mod (thunk () (mod (self :int)))
            :wis-mod (thunk () (mod (self :wis)))
            :cha-mod (thunk () (mod (self :cha)))
        ))
    ))
    dwarf)
    fighter
))

result


# (let :stat 12)
# (let :stats [ 3 5 7 9 11 13 15 17 ])
# (let :enumerated-stats (list/enumerate stats ))
# (let :mapped-stats (list/map
#     enumerated-stats
#     (fn ((:idx :val)) (tuple (- idx 4) val))
# ))
# (let :found-stats (list/find-or
#     mapped-stats
#     (fn ((:idx :el)) (<= stat el))
#     (tuple 4 18)
# ))
# found-stats
