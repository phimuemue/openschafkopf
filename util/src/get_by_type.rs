pub trait GetByType<T, const IDX: usize> {
    fn get_by_type(self) -> T;
}

pub fn get_by_type<T, const IDX: usize>(tuple: impl GetByType<T, IDX>) -> T { // TODO eliminate IDX
    tuple.get_by_type()
}

macro_rules! ignore_ident{($ident:ident, $($tt:tt)*) => { $($tt)* }}

macro_rules! impl_for_tuples_of_certain_length {
    ($($component:ident)+) => {
        impl_for_tuples_of_certain_length!(@p1 /*pre*/[] /*current+post*/[$($component)+]);
    };
    (@p1 [$($pre:ident)*] []) => {};
    (@p1 [$($pre:ident)*] [$cur:ident $($post:ident)*]) => {
        impl<$($pre,)* $cur, $($post,)*> GetByType<$cur, { 0 $(+ ignore_ident!($pre, 1))* }> for ($($pre,)* $cur, $($post,)*) {
            fn get_by_type(self) -> $cur {
                let ($(ignore_ident!($pre, _),)* x, $(ignore_ident!($post, _),)*) = self;
                x
            }
        }
        impl_for_tuples_of_certain_length!(@p1 [$($pre)* $cur] [$($post)*]);
    };
}

impl_for_tuples_of_certain_length!(T0);
impl_for_tuples_of_certain_length!(T0 T1);
