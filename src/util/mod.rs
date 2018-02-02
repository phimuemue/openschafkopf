pub use as_num::AsNum;
pub use plain_enum::*;
#[macro_use]
pub mod verify;
pub use self::verify::*;
#[macro_use]
pub mod box_clone;
pub use self::box_clone::*;
pub use failure::Error;
#[macro_use]
pub mod staticvalue;
pub use self::staticvalue::*;

// TODORUST static_assert not available in rust
macro_rules! static_assert{($assert_name:ident($($args:tt)*)) => {
    $assert_name!($($args)*)
}}

// TODORUST return impl
macro_rules! return_impl{($t:ty) => { $t }}
