(let :fix
    (fn (:f) (do
        (let* :result (f result))
        result
    ))
)

(let :result (fix (fn (:self)
    (obj/plain
    :x 1
    :y (thunk () (self :x))
    )
)))
