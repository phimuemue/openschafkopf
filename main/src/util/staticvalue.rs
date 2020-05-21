use std::fmt;

// TODORUST this should become superfluous once we have const generics
pub trait TStaticValue<
    V: Copy/*prevent interior mutation (suggested by clippy)*/
> : Sync + 'static + Clone + fmt::Debug {
    const VALUE : V;
}

macro_rules! define_static_value {
    (pub $struct: ident, $type: ty, $value: expr) => {
        #[derive(Copy, Clone, Debug)]
        pub struct $struct {}
        impl TStaticValue<$type> for $struct {
            const VALUE: $type = $value;
        }
        impl TStaticOrDynamicValue<$type> for $struct { // TODO can this be implemented outside of macro?
            fn value(self) -> $type {
                $value
            }
        }
    };
}

pub trait TStaticOrDynamicValue<T> {
    fn value(self) -> T;
}

impl<T> TStaticOrDynamicValue<T> for T {
    fn value(self) -> T {
        self
    }
}
