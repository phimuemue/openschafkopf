pub trait UnpackAndApplyFn<Args, Return> {
    fn apply(self, args: Args) -> Return;
}
macro_rules! impl_unpack_and_apply_fn{($($T:ident)*) => {
    impl<$($T,)* R, F: FnOnce($($T,)*)->R> UnpackAndApplyFn<($($T,)*), R> for F {
        #[allow(non_snake_case)]
        fn apply(self, ($($T,)*): ($($T,)*)) -> R {
            self($($T,)*)
        }
    }
}}
impl_unpack_and_apply_fn!();
impl_unpack_and_apply_fn!(T0);
impl_unpack_and_apply_fn!(T0 T1);
impl_unpack_and_apply_fn!(T0 T1 T2 T3);
pub fn make_const_unpack_and_apply<Args, Return>(r: Return) -> impl UnpackAndApplyFn<Args, Return> {
    struct SConstUnpackAndApply<Return>(Return);
    impl<Args, Return> UnpackAndApplyFn<Args, Return> for SConstUnpackAndApply<Return> {
        fn apply(self, _args: Args) -> Return {
            self.0
        }
    }
    SConstUnpackAndApply(r)
}


