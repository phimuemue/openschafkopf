pub use as_num::AsNum;
pub use plain_enum::*;
#[macro_use]
pub mod staticvalue;
pub mod interval;
pub mod vecext;
pub mod negext;
pub use self::{staticvalue::*, interval::*, vecext::*, negext::*};
pub use derive_new::new;
pub use failure::{bail, format_err, Error};
pub use openschafkopf_util::*;
#[macro_use]
pub mod bitfield;
#[macro_use]
pub mod dbg_argument;

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

