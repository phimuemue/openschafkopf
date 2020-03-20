#[macro_use]
pub mod verify;
pub use self::verify::*;
pub mod iteratorext;
pub use self::iteratorext::*;
pub mod via_out_param;
pub use self::via_out_param::*;
#[macro_use]
pub mod mutate_return;
pub use mutate_return::*;

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! if_dbg_else {({$($tt_dbg: tt)*}{$($tt_else: tt)*}) => {
    $($tt_dbg)*
}}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! if_dbg_else {({$($tt_dbg: tt)*}{$($tt_else: tt)*}) => {
    $($tt_else)*
}}

