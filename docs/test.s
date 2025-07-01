(let :id (fn (:x) x))
(let :fix-super (fn (:f :super) (do (let* :result (f result super)) result )))
(let :fix-record (fn (:f) (do (let* :result (f result (obj/plain))) result )))
(let :extend (fn (:base :ext) (do (fix-super ext base) )))
(let :to-record (fn (:plain) (fn (:self :super) plain) ))
(let :extend-fn (fn (:base :ext) (do (to-record (extend base ext)) )))

(let :first (obj/plain
    :a 1
))

(let :second (id (obj/record
    :a (+ (super :a) 1)
    :b 2
)))

(let :third (id (obj/record
    :a (+ (super :a) 2)
    :b (+ (super :b) 1)
)))


(let :result (extend (extend first second) third))
result
