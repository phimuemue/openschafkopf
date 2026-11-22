pub trait InternalGetByType<T, const IDX: usize> {
    fn internal_get_by_type(self) -> T;
    fn get_ref_by_type(&self) -> &T;
}
pub trait GetByType {
    fn get_by_type<T, const IDX: usize>(self) -> T
        where Self: InternalGetByType<T, IDX> + Sized
    {
        get_by_type::<T, IDX>(self)
    }
    fn get_ref_by_type<T, const IDX: usize>(&self) -> &T
        where Self: InternalGetByType<T, IDX> + Sized
    {
        get_ref_by_type::<T, IDX>(self)
    }
}

pub fn get_by_type<T, const IDX: usize>(tuple: impl InternalGetByType<T, IDX>) -> T { // TODO eliminate IDX
    tuple.internal_get_by_type()
}
pub fn get_ref_by_type<T, const IDX: usize>(tuple: &impl InternalGetByType<T, IDX>) -> &T { // TODO eliminate IDX
    tuple.get_ref_by_type()
}

macro_rules! ignore_ident{($ident:ident, $($tt:tt)*) => { $($tt)* }}

macro_rules! impl_for_tuples_of_certain_length {
    ($($component:ident)+) => {
        impl<$($component,)+> GetByType for ($($component,)+) {
        }
        impl_for_tuples_of_certain_length!(@p1 /*pre*/[] /*current+post*/[$($component)+]);
    };
    (@p1 [$($pre:ident)*] []) => {};
    (@p1 [$($pre:ident)*] [$cur:ident $($post:ident)*]) => {
        impl<$($pre,)* $cur, $($post,)*> InternalGetByType<$cur, { 0 $(+ ignore_ident!($pre, 1))* }> for ($($pre,)* $cur, $($post,)*) {
            fn internal_get_by_type(self) -> $cur {
                let ($(ignore_ident!($pre, _),)* x, $(ignore_ident!($post, _),)*) = self;
                x
            }
            fn get_ref_by_type(&self) -> &$cur {
                let ($(ignore_ident!($pre, _),)* x, $(ignore_ident!($post, _),)*) = self;
                x
            }
        }
        impl_for_tuples_of_certain_length!(@p1 [$($pre)* $cur] [$($post)*]);
    };
}

impl_for_tuples_of_certain_length!(T0);
impl_for_tuples_of_certain_length!(T0 T1);
impl_for_tuples_of_certain_length!(T0 T1 T2);
