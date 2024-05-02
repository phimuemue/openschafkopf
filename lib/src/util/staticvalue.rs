macro_rules! define_static_value {
    (pub $struct: ident, $type: ty, $value: expr) => {
        #[derive(Copy, Clone, Debug, Default)]
        pub struct $struct;
        impl TStaticOrDynamicValue<$type> for $struct {
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
