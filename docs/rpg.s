(let :fix-super (fn (:f :super) (do
    (let* :result (f result super))
    result
)))

(let :extend-root (fn (:root :base :ext) (do
    (let :plain (obj/plain))
    (let*
                      # self root super
        :super  (base result root plain)
        :result (ext result root super)
    )
    result
)))

(let :extend (fn (:base :ext) (do
    (let :plain (obj/plain))
    (let*
        :super  (base result result plain)
        :result (ext result result super)
    )
    result
)))

(let :to-record (fn (:plain)
    (fn (:self :root :super) plain)
))

(let :extend-fn (fn (:base :ext) (do
    (to-record (extend base ext))
)))


(tuple
    fix-super
    extend-root
    extend
    to-record
    extend-fn
)
