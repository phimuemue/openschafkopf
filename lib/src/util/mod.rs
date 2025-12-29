pub use as_num::AsNum;
pub use plain_enum::*;
#[macro_use]
pub mod staticvalue;
pub mod vecext;
pub mod negext;
pub mod optionext;
pub mod static_option;
pub use self::{staticvalue::*, vecext::*, negext::*, optionext::*, static_option::*};
pub use derive_new::new;
pub use failure::{bail, format_err, Error};
pub use openschafkopf_util::*;
#[macro_use]
pub mod bitfield;
#[macro_use]
pub mod dbg_argument;
#[macro_use]
pub mod forward_to_field;

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

