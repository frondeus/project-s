use super::*;

trait TupleLen {
    const LEN: usize;
}

impl TupleLen for () {
    const LEN: usize = 0;
}
// impl<T> TupleLen for (T,) { const LEN: usize = 1;}
macro_rules! tuple_len {
() => {};
($first: tt  $(,$arg: tt)*) => {

    tuple_len!($($arg),*);
    impl<$first, $($arg),*> TupleLen for ($first, $($arg,)*) { const LEN: usize = 1 + <($($arg,)*) as TupleLen>::LEN; }
};
}
tuple_len!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

//------------- Implementations ------------

// Possible cases:
// 1. With runtime prefixes
// 2. With arguments in the middle
// 3. With rest at the end

pub struct WithRuntime;
pub struct WithRest;
// struct WithArgs;

macro_rules! fnlike {
// 1. Without runtime or rest
(NO,) => {
    impl<F, O> IntoNativeFunction<(O,)> for F
    where F: 'static + Fn() -> O,
        O: IntoValue + 'static,
    {
        fn into_native_function(self) -> Box<dyn NativeFunction> {
            FnLike::<F, (O,)>::box_new(self)
        }
    }

    impl<F, O> NativeFunction for FnLike<F, (O,)>
    where F: 'static + Fn() -> O,
        O: IntoValue,
    {
        fn signature_matches(&self, _rt: &mut Runtime, _values: &[Value]) -> bool {
            true
        }

        fn try_call(&self, rt: &mut Runtime, _values: Vec<Value>) -> Result<Value, String> {
            let o = (self.f)();
            O::try_into_value(o, rt)
        }

        fn with_name(&mut self, name: &'static str) {
            self.name = Some(name);
        }
    }
};

(NO, $first: tt, $($arg: tt,)*) => {
    fnlike!(NO, $($arg,)*);

    impl<F, O, $first $(,$arg)*> IntoNativeFunction<(O, $first, $($arg),*)> for F
    where
        F: 'static + Fn($first, $($arg),*) -> O,
        O: IntoValue + 'static,
        $first: FromValue + 'static,
        $($arg: FromValue + 'static),*
    {
        fn into_native_function(self) -> Box<dyn NativeFunction> {
            FnLike::<F, (O, $first, $($arg),*)>::box_new(self)
        }
    }

    #[allow(non_snake_case)]
    impl<F, O, $first, $($arg),*> NativeFunction for FnLike<F, (O, $first, $($arg),*)>
    where
        F: 'static + Fn($first, $($arg),*) -> O,
        O: IntoValue,
        $first: FromValue,
        $($arg: FromValue),*
    {
        fn signature_matches(&self, rt: &mut Runtime, values: &[Value]) -> bool {
            if !has_arity(<($first, $($arg),*) as TupleLen>::LEN, values) {
                return false;
            }

            let mut values = values.into_iter();

            if !$first::is_matching(rt, values.next().unwrap()) {
                return false;
            }
            $(
                if !$arg::is_matching(rt, values.next().unwrap()) {
                    return false;
                }
            )*

            true
        }

        fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
            assert_arity(<($first, $($arg),*) as TupleLen>::LEN, &values)?;
            let mut values = values.into_iter();

            let $first = $first::try_from_value(rt, values.next().unwrap())?;
            $(
                let $arg = $arg::try_from_value(rt, values.next().unwrap())?;
            )*

            let o = (self.f)($first, $($arg,)*);

            O::try_into_value(o, rt)
        }

        fn with_name(&mut self, name: &'static str) {
            self.name = Some(name);
        }
    }


};


// 2. With runtime, no rest
(RT ,) => {
    impl<F, O> IntoNativeFunction<(O, WithRuntime)> for F
    where F: 'static + Fn(&mut Runtime) -> O,
        O: IntoValue + 'static,
    {
        fn into_native_function(self) -> Box<dyn NativeFunction> {
            FnLike::<F, (O, WithRuntime)>::box_new(self)
        }
    }

    impl<F, O> NativeFunction for FnLike<F, (O, WithRuntime)>
    where F: 'static + Fn(&mut Runtime) -> O,
        O: IntoValue,
    {
        fn signature_matches(&self, _rt: &mut Runtime, _values: &[Value]) -> bool {
            true
        }
        fn try_call(&self, rt: &mut Runtime, _values: Vec<Value>) -> Result<Value, String> {
            let o = (self.f)(rt);
            O::try_into_value(o, rt)
        }

        fn with_name(&mut self, name: &'static str) {
            self.name = Some(name);
        }
    }

};

(RT, $first: tt, $($arg: tt,)*) => {
    fnlike!(RT, $($arg,)*);

    impl<F, O, $first, $($arg),*> IntoNativeFunction<(O, $first, $($arg,)* WithRuntime)> for F
    where
        F: 'static + Fn(&mut Runtime, $first, $($arg),*) -> O,
        O: IntoValue + 'static,
        $first: FromValue + 'static,
        $($arg: FromValue + 'static),*
    {
        fn into_native_function(self) -> Box<dyn NativeFunction> {
            FnLike::<F, (O, $first, $($arg,)* WithRuntime)>::box_new(self)
        }
    }

    #[allow(non_snake_case)]
    impl<F, O, $first, $($arg),*> NativeFunction for FnLike<F, (O, $first, $($arg,)* WithRuntime)>
    where
        F: 'static + Fn(&mut Runtime, $first, $($arg),*) -> O,
        O: IntoValue,
        $first: FromValue,
        $($arg: FromValue),*
    {

        fn with_name(&mut self, name: &'static str) {
            self.name = Some(name);
        }

        fn signature_matches(&self, rt: &mut Runtime, values: &[Value]) -> bool {
            if !has_arity(<($first, $($arg),*) as TupleLen>::LEN, values) {
                return false;
            }

            let mut values = values.into_iter();

            if !$first::is_matching(rt, values.next().unwrap()) {
                return false;
            }

            $(
                if !$arg::is_matching(rt, values.next().unwrap()) {
                    return false;
                }
            )*

            true
        }

        fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
            assert_arity(<($first, $($arg),*) as TupleLen>::LEN, &values)?;
            let mut values = values.into_iter();

            let $first = $first::try_from_value(rt, values.next().unwrap())?;
            $(
                let $arg = $arg::try_from_value(rt, values.next().unwrap())?;
            )*

            let o = (self.f)(rt, $first, $($arg,)*);

            O::try_into_value(o, rt)
        }
    }

};

// 3. With rest no runtime

(RE ,) => {
    impl<F, O, R> IntoNativeFunction<(O, R, WithRest)> for F
    where
        F: 'static + Fn(Rest<R>) -> O,
        O: IntoValue + 'static,
        R: FromValue + 'static,
    {
        fn into_native_function(self) -> Box<dyn NativeFunction> {
            FnLike::<F, (O, R, WithRest)>::box_new(self)
        }
    }

    impl<F, O, R> NativeFunction for FnLike<F, (O, R, WithRest)>
    where
        F: 'static + Fn(Rest<R>) -> O,
        O: IntoValue,
        R: FromValue,
    {
        fn with_name(&mut self, name: &'static str) {
            self.name = Some(name);
        }

        fn signature_matches(&self, rt: &mut Runtime, values: &[Value]) -> bool {
            Rest::<R>::signature_matches(rt, values)
        }

        fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
            let rest = Rest { values };
            let rest = Rest::<R>::try_from_values(rt, rest)?;
            let o = (self.f)(rest);
            O::try_into_value(o, rt)
        }
    }
};

(RE, $first: tt, $($arg: tt,)*) => {
    fnlike!(RE, $($arg,)*);

    impl<F, O, $first, $($arg,)* R> IntoNativeFunction<(O, $first, $($arg,)* R, WithRest)> for F
    where
        F: 'static + Fn($first, $($arg,)* Rest<R>) -> O,
        O: IntoValue + 'static,
        R: FromValue + 'static,
        $first: FromValue + 'static,
        $($arg: FromValue + 'static),*
    {
        fn into_native_function(self) -> Box<dyn NativeFunction> {
            FnLike::<F, (O, $first, $($arg,)* R, WithRest)>::box_new(self)
        }
    }

    #[allow(non_snake_case)]
    impl<F, O, $first, $($arg,)* R> NativeFunction for FnLike<F, (O, $first, $($arg,)* R, WithRest)>
    where
        F: 'static + Fn($first, $($arg,)* Rest<R>) -> O,
        O: IntoValue,
        R: FromValue,
        $first: FromValue,
        $($arg: FromValue),*
    {

        fn with_name(&mut self, name: &'static str) {
            self.name = Some(name);
        }

        fn signature_matches(&self, rt: &mut Runtime, values: &[Value]) -> bool {
            let len = <($first, $($arg),*) as TupleLen>::LEN;
            if !has_at_least_arity(len, values) {
                return false;
            }

            let rest = &values[len..];

            Rest::<R>::signature_matches(rt, rest)
        }

        fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
            assert_at_least_arity(<($first, $($arg),*) as TupleLen>::LEN, &values)?;

            let mut values = values.into_iter();

            let $first = $first::try_from_value(rt, values.next().unwrap())?;
            $(
                let $arg = $arg::try_from_value(rt, values.next().unwrap())?;
            )*

            let rest = Rest { values: values.collect() };

            let rest = Rest::<R>::try_from_values(rt, rest)?;
            let o = (self.f)($first, $($arg,)* rest);

            O::try_into_value(o, rt)
        }
    }

};

// 4. With runtime and rest

(RTRE ,) => {
    impl<F, O, R> IntoNativeFunction<(O, R, WithRuntime, WithRest)> for F
    where
        F: 'static + Fn(&mut Runtime, Rest<R>) -> O,
        O: IntoValue + 'static,
        R: FromValue + 'static,
    {
        fn into_native_function(self) -> Box<dyn NativeFunction> {
            FnLike::<F, (O, R, WithRuntime, WithRest)>::box_new(self)
        }
    }

    impl<F, O, R> NativeFunction for FnLike<F, (O, R, WithRuntime, WithRest)>
    where
        F: 'static + Fn(&mut Runtime, Rest<R>) -> O,
        O: IntoValue,
        R: FromValue,
    {
        fn with_name(&mut self, name: &'static str) {
            self.name = Some(name);
        }

        fn signature_matches(&self, rt: &mut Runtime, values: &[Value]) -> bool {
            Rest::<R>::signature_matches(rt, values)
        }
        fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
            let rest = Rest { values };
            let rest = Rest::<R>::try_from_values(rt, rest)?;
            let o = (self.f)(rt, rest);
            O::try_into_value(o, rt)
        }
    }
};

(RTRE, $first: tt, $($arg: tt,)*) => {
    fnlike!(RTRE, $($arg,)*);

    impl<F, O, $first, $($arg,)* R> IntoNativeFunction<(O, $first, $($arg,)* R, WithRuntime, WithRest)> for F
    where
        F: 'static + Fn(&mut Runtime, $first, $($arg,)* Rest<R>) -> O,
        O: IntoValue + 'static,
        R: FromValue + 'static,
        $first: FromValue + 'static,
        $($arg: FromValue + 'static),*
    {
        fn into_native_function(self) -> Box<dyn NativeFunction> {
            FnLike::<F, (O, $first, $($arg,)* R, WithRuntime, WithRest)>::box_new(self)
        }
    }

    #[allow(non_snake_case)]
    impl<F, O, $first, $($arg,)* R> NativeFunction for FnLike<F, (O, $first, $($arg,)* R, WithRuntime, WithRest)>
    where
        F: 'static + Fn(&mut Runtime, $first, $($arg,)* Rest<R>) -> O,
        O: IntoValue,
        R: FromValue,
        $first: FromValue,
        $($arg: FromValue),*
    {
        fn with_name(&mut self, name: &'static str) {
            self.name = Some(name);
        }

        fn signature_matches(&self, rt: &mut Runtime, values: &[Value]) -> bool {
            let len = <($first, $($arg),*) as TupleLen>::LEN;
            if !has_at_least_arity(len, values) {
                return false;
            }

            let rest = &values[len..];
            Rest::<R>::signature_matches(rt, rest)
        }

        fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
            assert_at_least_arity(<($first, $($arg),*) as TupleLen>::LEN, &values)?;

            let mut values = values.into_iter();

            let $first = $first::try_from_value(rt, values.next().unwrap())?;
            $(
                let $arg = $arg::try_from_value(rt, values.next().unwrap())?;
            )*

            let rest = Rest { values: values.collect() };
            let rest = Rest::<R>::try_from_values(rt, rest)?;
            let o = (self.f)(rt, $first, $($arg,)* rest);

            O::try_into_value(o, rt)
        }
    }

};
}

fnlike!(
    NO, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16,
);
fnlike!(
    RT, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16,
);
fnlike!(
    RE, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16,
);
fnlike!(
    RTRE, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16,
);

// AllParams (WithParams) support - no runtime
impl<F, O, R> IntoNativeFunction<(O, R, WithParams)> for F
where
    F: 'static + Fn(AllParams<R>) -> O,
    O: IntoValue + 'static,
    R: 'static,
{
    fn into_native_function(self) -> Box<dyn NativeFunction> {
        FnLike::<F, (O, R, WithParams)>::box_new(self)
    }
}

impl<F, O, R> NativeFunction for FnLike<F, (O, R, WithParams)>
where
    F: 'static + Fn(AllParams<R>) -> O,
    O: IntoValue,
    R: 'static,
{
    fn with_name(&mut self, name: &'static str) {
        self.name = Some(name);
    }

    fn signature_matches(&self, _rt: &mut Runtime, _values: &[Value]) -> bool {
        true
    }

    fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
        let o = (self.f)(AllParams::new(values));
        O::try_into_value(o, rt)
    }
}

// AllParams (WithParams) support - with runtime
impl<F, O, R> IntoNativeFunction<(O, R, WithRuntime, WithParams)> for F
where
    F: 'static + Fn(&mut Runtime, AllParams<R>) -> O,
    O: IntoValue + 'static,
    R: 'static,
{
    fn into_native_function(self) -> Box<dyn NativeFunction> {
        FnLike::<F, (O, R, WithRuntime, WithParams)>::box_new(self)
    }
}

impl<F, O, R> NativeFunction for FnLike<F, (O, R, WithRuntime, WithParams)>
where
    F: 'static + Fn(&mut Runtime, AllParams<R>) -> O,
    O: IntoValue,
    R: 'static,
{
    fn with_name(&mut self, name: &'static str) {
        self.name = Some(name);
    }

    fn signature_matches(&self, _rt: &mut Runtime, _values: &[Value]) -> bool {
        true
    }

    fn try_call(&self, rt: &mut Runtime, values: Vec<Value>) -> Result<Value, String> {
        let o = (self.f)(rt, AllParams::new(values));
        O::try_into_value(o, rt)
    }
}
