use std::fmt;

// TODORUST this should become superfluous once we have const generics
pub trait TStaticValue<V> : Sync + 'static + Clone + fmt::Debug
    where V: Copy, // prevent interior mutation (suggested by clippy)
{
    const VALUE : V;
}

macro_rules! define_static_value {(pub $struct: ident, $type: ty, $value: expr) => {
    #[derive(Clone, Debug)]
    pub struct $struct {}
    impl TStaticValue<$type> for $struct {
        const VALUE : $type = $value;
    }
}}
