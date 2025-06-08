// (self, root, super_)
let create_obj = (constructor) => {
//   return (root, super_, maybe_self) => {
  return (ctx) => {
    let self = ctx.self ?? new Map();
    let root = ctx.root ?? self;
    let super_ = ctx.super_;
    if (constructor instanceof Function) {
        constructor({self, root, super_});
    }
    if (constructor instanceof Object) {
        for (let [key, value] of Object.entries(constructor)) {
            self.set(key, value);
        }
    }
    return Object.fromEntries(self);
  };
};

let add = (a, b) => create_obj(({self, root}) => {
    let left = a({ root, super_: self, self });
    let super_ = new Map(Object.entries(left));
    b({ root: undefined, super_, self });
});

let output = (obj) => {
    let result = obj({});
    return JSON.stringify(result, null, "  ");
};


module.exports = { create_obj, add, output };