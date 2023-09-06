pub use as_num::AsNum;
pub use plain_enum::*;
#[macro_use]
pub mod box_clone;
#[macro_use]
pub mod staticvalue;
pub mod interval;
pub mod vecext;
pub mod negext;
pub use self::{box_clone::*, staticvalue::*, interval::*, if_then::*, vecext::*, negext::*, dbg_argument::*};
pub use derive_new::new;
pub use failure::{bail, format_err, Error};
pub use openschafkopf_util::*;
#[macro_use]
pub mod bitfield;
#[macro_use]
pub mod dbg_argument;

// TODORUST Objects should be upcastable to supertraits: https://github.com/rust-lang/rust/issues/5665
macro_rules! make_upcastable {
    ($upcasttrait:ident, $trait:ident) => {
        pub trait $upcasttrait {
            fn upcast(&self) -> &dyn $trait;
            fn upcast_box(self: Box<Self>) -> Box<dyn $trait>
            where
                Self: 'static;
        }
        impl<T: $trait> $upcasttrait for T {
            fn upcast(&self) -> &dyn $trait {
                self
            }
            fn upcast_box(self: Box<Self>) -> Box<dyn $trait>
            where
                Self: 'static,
            {
                self as Box<dyn $trait>
            }
        }
    };
}

pub fn tpl_flip_if<T>(b: bool, (t0, t1): (T, T)) -> (T, T) {
    if b {
        (t1, t0)
    } else {
        (t0, t1)
    }
}

macro_rules! type_dispatch_enum{(pub enum $e: ident {$($v: ident ($t: ty),)+}) => {
    pub enum $e {
        $($v($t),)+
    }
    $(
        impl From<$t> for $e {
            fn from(t: $t) -> Self {
                $e::$v(t)
            }
        }
    )+
}}

