pub use as_num::AsNum;
pub use plain_enum::*;
#[macro_use]
pub mod staticvalue;
pub mod vecext;
pub mod optionext;
pub mod static_option;
pub mod int_ext;
pub use self::{staticvalue::*, vecext::*, optionext::*, static_option::*, int_ext::*};
pub use derive_new::new;
pub use failure::{format_err, Error};
pub use openschafkopf_util::*;
#[macro_use]
pub mod bitfield;
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

