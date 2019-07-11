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

macro_rules! if_then_some{($cond: expr, $val: expr) => {
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
    fn is_sorted_unstable_name_collision(self) -> bool
        where
            Self: Sized,
            Self::Item: PartialOrd,
    {
        self.is_sorted_by_unstable_name_collision(|a, b| a.partial_cmp(b))
    }

    fn is_sorted_by_unstable_name_collision<F>(mut self, mut compare: F) -> bool
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

    fn is_sorted_by_key_unstable_name_collision<F, K>(self, mut f: F) -> bool
        where
            Self: Sized,
            F: FnMut(&Self::Item) -> K,
            K: PartialOrd
    {
        self.is_sorted_by_unstable_name_collision(|a, b| f(a).partial_cmp(&f(b)))
    }

    // TODO this should be part of itertools (https://github.com/bluss/rust-itertools/issues/334)
    fn single(&mut self) -> Result<Self::Item, ESingleError> {
        match self.next() {
            None => Err(ESingleError::Empty),
            Some(element) => {
                match self.next() {
                    None => Ok(element),
                    Some(_) => Err(ESingleError::MoreThanOne),
                }
            }
        }
    }

    fn fold_mutating<B, F: FnMut(&mut B, Self::Item)>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
    {
        self.fold(init, move |mut b, item| {
            f(&mut b, item);
            b
        })
    }
}

impl<It> IteratorExt for It where It: Iterator {}

#[derive(Debug)]
pub enum ESingleError {Empty, MoreThanOne}

macro_rules! cartesian_match(
    (
        $macro_callback: ident,
        $(match ($e: expr) {
            $($x: pat $(| $xs: pat)* => $y: tt,)*
        },)*
    ) => {
        cartesian_match!(@p0,
            $macro_callback,
            (),
            $(match ($e) {
                $($x $(| $xs)* => $y,)*
            },)*
        )
    };
    (@p0,
        $macro_callback: ident,
        $rest_packed: tt,
        match ($e: expr) {
            $($x: pat $(| $xs: pat)* => $y: tt,)*
        },
        $(match ($e2: expr) {
            $($x2: pat $(| $xs2: pat)* => $y2: tt,)*
        },)*
    ) => {
        cartesian_match!(@p0,
            $macro_callback,
            (
                match ($e) {
                    $($x $(| $xs)* => $y,)*
                },
                $rest_packed,
            ),
            $(match ($e2) {
                $($x2 $(| $xs2)* => $y2,)*
            },)*
        )
    };
    (@p0,
        $macro_callback: ident,
        $rest_packed: tt,
    ) => {
        cartesian_match!(@p1,
            $macro_callback,
            @matched{()},
            $rest_packed,
        )
    };
    (@p1,
        $macro_callback: ident,
        @matched{$matched_packed: tt},
        (
            match ($e: expr) {
                $($x: pat $(| $xs: pat)* => $y: tt,)*
            },
            $rest_packed: tt,
        ),
    ) => {
        match $e {
            $($x $(| $xs)* => cartesian_match!(@p1,
                $macro_callback,
                @matched{ ($matched_packed, $y,) },
                $rest_packed,
            ),)*
        }
    };
    (@p1,
        $macro_callback: ident,
        @matched{$matched_packed: tt},
        (),
    ) => {
        cartesian_match!(@p2,
            $macro_callback,
            @unpacked(),
            $matched_packed,
        )
        //$macro_callback!($matched_packed)
    };
    (@p2,
        $macro_callback: ident,
        @unpacked($($u: tt,)*),
        (
            $rest_packed: tt,
            $y: tt,
        ),
    ) => {
        cartesian_match!(@p2,
            $macro_callback,
            @unpacked($($u,)* $y,),
            $rest_packed,
        )
    };
    (@p2,
        $macro_callback: ident,
        @unpacked($($u: tt,)*),
        (),
    ) => {
        $macro_callback!($($u,)*)
    };
);

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
