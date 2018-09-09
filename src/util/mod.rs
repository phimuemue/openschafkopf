pub use as_num::AsNum;
pub use plain_enum::*;
#[macro_use]
pub mod verify;
#[macro_use]
pub mod box_clone;
#[macro_use]
pub mod staticvalue;
pub use self::{
    verify::*,
    box_clone::*,
    staticvalue::*,
};
pub use failure::Error;

// TODORUST static_assert not available in rust
macro_rules! static_assert{($assert_name:ident($($args:tt)*)) => {
    $assert_name!($($args)*)
}}

// TODORUST return impl
macro_rules! return_impl{($t:ty) => { $t }}

// TODORUST Objects should be upcastable to supertraits: https://github.com/rust-lang/rust/issues/5665
macro_rules! make_upcastable{($upcasttrait:ident, $trait:ident) => {
    pub trait $upcasttrait {
        fn upcast(&self) -> &$trait;
    }
    impl<T: $trait> $upcasttrait for T {
        fn upcast(&self) -> &$trait {
            self
        }
    }
}}

macro_rules! if_then_option{($cond: expr, $val: expr) => {
    if $cond {Some($val)} else {None}
}}
