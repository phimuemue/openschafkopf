#[macro_use]
pub mod verify;
pub use self::verify::*;
pub mod iteratorext;
pub use self::iteratorext::*;

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

