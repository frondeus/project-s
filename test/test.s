
(let :expand (fn (:base :ext) (do
    (let*
        :super  (base result)
        :result (ext result super)
    )
    result
)))

(let :left (fn (:self) (obj/plain
    :x 1
    :y (thunk () (self :x))
)))

(let :right (fn (:self :super) (obj/extend super
    :x 2
)))

(expand left right)
