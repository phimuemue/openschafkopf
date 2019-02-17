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
        fn upcast(&self) -> &dyn $trait;
    }
    impl<T: $trait> $upcasttrait for T {
        fn upcast(&self) -> &dyn $trait {
            self
        }
    }
}}

macro_rules! if_then_option{($cond: expr, $val: expr) => {
    if $cond {Some($val)} else {None}
}}

pub fn tpl_flip_if<T>(b: bool, (t0, t1): (T, T)) -> (T, T) {
    if b {
        (t1, t0)
    } else {
        (t0, t1)
    }
}

#[cfg(debug_assertions)]
macro_rules! if_dbg_else {({$($tt_dbg: tt)*}{$($tt_else: tt)*}) => {
    $($tt_dbg)*
}}
#[cfg(not(debug_assertions))]
macro_rules! if_dbg_else {({$($tt_dbg: tt)*}{$($tt_else: tt)*}) => {
    $($tt_else)*
}}

// TODORUST is_sorted et al. should be part of std:
// https://github.com/rust-lang/rfcs/pull/2351,
// https://github.com/rust-lang/rfcs/blob/master/text/2351-is-sorted.md
// https://github.com/rust-lang/rust/issues/53485
// For now, use implementation from https://github.com/rust-lang/rust/blob/b5ab524ea7b536617d8abc5507a1d97b3e60a42d/src/libcore/iter/iterator.rs
pub trait IteratorExt : Iterator {
    fn is_sorted(self) -> bool
        where
            Self: Sized,
            Self::Item: PartialOrd,
    {
        self.is_sorted_by(|a, b| a.partial_cmp(b))
    }

    fn is_sorted_by<F>(mut self, mut compare: F) -> bool
        where
            Self: Sized,
            F: FnMut(&Self::Item, &Self::Item) -> Option<std::cmp::Ordering>
    {
        let mut last = match self.next() {
            Some(e) => e,
            None => return true,
        };
        for curr in self {
            if compare(&last, &curr).map(|o| o == std::cmp::Ordering::Greater).unwrap_or(true) {
                return false;
            }
            last = curr;
        }
        true
    }

    fn is_sorted_by_key<F, K>(self, mut f: F) -> bool
        where
            Self: Sized,
            F: FnMut(&Self::Item) -> K,
            K: PartialOrd
    {
        self.is_sorted_by(|a, b| f(a).partial_cmp(&f(b)))
    }
}

impl<It> IteratorExt for It where It: Iterator {}
