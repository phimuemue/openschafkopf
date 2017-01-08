use std::mem;
use std::fmt::Debug;

// heavily inspired by http://rust-num.github.io/num/src/num_traits/cast.rs.html

// TODO rust i128/u128
type LargestSignedType = i64;
type LargestUnsignedType = u64;

pub trait TSignedInt : Sized + Copy {
    fn min() -> LargestSignedType;
    fn max() -> LargestSignedType;
}

pub trait TUnsignedInt : Sized + Copy {
    fn min() -> LargestUnsignedType;
    fn max() -> LargestUnsignedType;
}

macro_rules! impl_min_max {
    ($num_trait: ident, $largest_type_same_signedness: ty,) => {};
    ($num_trait: ident, $largest_type_same_signedness: ty, $t: ident, $($ts: ident,)*) => {
        impl $num_trait for $t {
            fn min() -> $largest_type_same_signedness {
                use std::$t;
                $t::MIN as $largest_type_same_signedness
            }
            fn max() -> $largest_type_same_signedness {
                use std::$t;
                $t::MAX as $largest_type_same_signedness
            }
        }
        impl_min_max!($num_trait, $largest_type_same_signedness, $($ts,)*);
    };
}

impl_min_max!(TSignedInt, LargestSignedType, i8, i16, i32, i64, isize,);
impl_min_max!(TUnsignedInt, LargestUnsignedType, u8, u16, u32, u64, usize,);

pub trait TAsNumInternal<Dest> : Copy {
    fn is_safely_convertible(self) -> bool;
    fn as_num_internal(self) -> Dest;
}

pub trait TAsNum : Copy {
    fn as_num<Dest>(self) -> Dest
        where Self: TAsNumInternal<Dest>,
              Dest: TAsNumInternal<Self>,
              Dest: Debug;
    fn checked_as_num<Dest>(self) -> Option<Dest>
        where Self: TAsNumInternal<Dest>,
              Dest: TAsNumInternal<Self>,
              Dest: Debug;
    fn assert_convertible_back<Dest>(self)
        where Self: TAsNumInternal<Dest>,
              Dest: TAsNumInternal<Self>,
              Dest: Debug;
}

macro_rules! impl_TAsNum {
    () => {};
    ($t: ident, $($ts: ident,)*) => {
        impl TAsNum for $t {
            fn assert_convertible_back<Dest>(self)
                where Self: TAsNumInternal<Dest>,
                      Dest: TAsNumInternal<Self>,
                      Dest: Debug,
            {
                let dst : Dest = self.as_num_internal();
                let src : Self = dst.as_num_internal();
                debug_assert!(self==src, "{:?} {:?} was converted to {:?}, whose back-conversion yields {:?}", self, stringify!($t), dst, src);
            }
            fn as_num<Dest>(self) -> Dest
                where Self: TAsNumInternal<Dest>,
                      Dest: TAsNumInternal<Self>,
                      Dest: Debug,
            {
                debug_assert!(self.is_safely_convertible());
                self.assert_convertible_back::<Dest>();
                self.as_num_internal()
            }
            fn checked_as_num<Dest>(self) -> Option<Dest>
                where Self: TAsNumInternal<Dest>,
                      Dest: TAsNumInternal<Self>,
                      Dest: Debug,
            {
                if self.is_safely_convertible() {
                    self.assert_convertible_back::<Dest>();
                    Some(self.as_num_internal())
                } else {
                    None
                }
            }
        }
        impl_TAsNum!($($ts,)*);
    };
}
impl_TAsNum!(
    i8, i16, i32, i64, isize,
    u8, u16, u32, u64, usize,
    f32, f64,
);

macro_rules! impl_signed_to_signed_internal {
    ($src: ident, $dest: ident) => {
        impl TAsNumInternal<$dest> for $src {
            fn is_safely_convertible(self) -> bool {
                mem::size_of::<$src>() <= mem::size_of::<$dest>()
                || {
                    debug_assert!(mem::size_of::<Self>() <= mem::size_of::<LargestSignedType>());
                    let n = self as LargestSignedType;
                    $dest::min() <= n && n <= $dest::max()
                }
            }
            fn as_num_internal(self) -> $dest {
                self as $dest
            }
        }
    };
}

macro_rules! impl_signed_to_signed {
    ($src: ident,) => {};
    ($src: ident, $dest: ident, $($dests: ident,)*) => {
        impl_signed_to_signed_internal!($src, $dest);
        impl_signed_to_signed_internal!($dest, $src);
        impl_signed_to_signed!($src, $($dests,)*);
    };
}

macro_rules! impl_signed_to_unsigned_internal {
    ($src: ident, $dest: ident) => {
        impl TAsNumInternal<$dest> for $src {
            fn is_safely_convertible(self) -> bool {
                0<=self && self as LargestUnsignedType <= $dest::max()
            }
            fn as_num_internal(self) -> $dest {
                self as $dest
            }
        }
    }
}

macro_rules! impl_signed_to_unsigned {
    ($src: ident,) => {};
    ($src: ident, $dest: ident, $($dests: ident,)*) => {
        impl_signed_to_unsigned_internal!($src, $dest);
        impl_unsigned_to_signed_internal!($dest, $src);
        impl_signed_to_unsigned!($src, $($dests,)*);
    }
}

macro_rules! impl_unsigned_to_signed_internal {
    ($src: ident, $dest: ident) => {
        impl TAsNumInternal<$dest> for $src {
            fn is_safely_convertible(self) -> bool {
                self as LargestSignedType <= $dest::max()
            }
            fn as_num_internal(self) -> $dest {
                self as $dest
            }
        }
    };
}

macro_rules! impl_unsigned_to_signed {
    ($src: ident,) => {};
    ($src: ident, $dest: ident, $($dests: ident,)*) => {
        impl_unsigned_to_signed_internal!($src, $dest);
        impl_signed_to_unsigned_internal!($dest, $src);
        impl_unsigned_to_signed!($src, $($dests,)*);
    };
}

macro_rules! impl_unsigned_to_unsigned_internal {
    ($src: ident, $dest: ident) => {
        impl TAsNumInternal<$dest> for $src {
            fn is_safely_convertible(self) -> bool {
                mem::size_of::<$src>() <= mem::size_of::<$dest>()
                    || self as LargestUnsignedType <= $dest::max()
            }
            fn as_num_internal(self) -> $dest {
                self as $dest
            }
        }
    };
}

macro_rules! impl_unsigned_to_unsigned {
    ($src: ident,) => {};
    ($src: ident, $dest: ident, $($dests: ident,)*) => {
        impl_unsigned_to_unsigned_internal!($src, $dest);
        impl_unsigned_to_unsigned_internal!($dest, $src);
        impl_unsigned_to_unsigned!($src, $($dests,)*);
    };
}

macro_rules! impl_integral_conversions {
    ((), ($($unsigneds: ident,)*)) => {};
    (($signed: ident, $($signeds: ident,)*), ($unsigned: ident, $($unsigneds: ident,)*)) => {
        impl_signed_to_signed_internal!($signed, $signed);
        impl_signed_to_signed!($signed, $($signeds,)*);
        impl_signed_to_unsigned_internal!($signed, $unsigned);
        impl_signed_to_unsigned!($signed, $($unsigneds,)*);
        impl_unsigned_to_signed_internal!($unsigned, $signed);
        impl_unsigned_to_signed!($unsigned, $($signeds,)*);
        impl_unsigned_to_unsigned_internal!($unsigned, $unsigned);
        impl_unsigned_to_unsigned!($unsigned, $($unsigneds,)*);
        impl_integral_conversions!(($($signeds,)*), ($($unsigneds,)*));
    };
}

impl_integral_conversions!(
    (i8, i16, i32, i64, isize,),
    (u8, u16, u32, u64, usize,)
);

macro_rules! impl_integral_to_float_internal {
    ($flt: ident,) => {};
    ($flt: ident, $int: ident, $($ints: ident,)*) => {
        impl TAsNumInternal<$flt> for $int {
            fn is_safely_convertible(self) -> bool {
                true // assume convertability until we encounter counter example in practice
            }
            fn as_num_internal(self) -> $flt {
                self as $flt
            }
        }
        impl TAsNumInternal<$int> for $flt {
            fn is_safely_convertible(self) -> bool {
                let dst : $int = self.as_num_internal();
                let src : Self = dst.as_num_internal();
                self==src
            }
            fn as_num_internal(self) -> $int {
                self as $int
            }
        }
        impl_integral_to_float_internal!($flt, $($ints,)*);
    };
}
macro_rules! impl_integral_to_float {
    ($flt: ident) => {
        impl_integral_to_float_internal!($flt,
            i8, i16, i32, i64, isize,
            u8, u16, u32, u64, usize,
        );
    };
}
impl_integral_to_float!(f32);
impl_integral_to_float!(f64);

type LargestFloatType = f64;
macro_rules! impl_float_to_float_internal {
    ($src: ident, $dest: ident) => {
        impl TAsNumInternal<$dest> for $src {
            fn is_safely_convertible(self) -> bool {
                mem::size_of::<$src>() <= mem::size_of::<$dest>() 
                || {
                    // Make sure the value is in range for the cast.
                    // NaN and +-inf are cast as they are.
                    let f = self as LargestFloatType;
                    !f.is_finite() || {
                        let max_value: $dest = ::std::$dest::MAX;
                        -max_value as LargestFloatType <= f && f <= max_value as LargestFloatType
                    }
                }
            }
            fn as_num_internal(self) -> $dest {
                self as $dest
            }
        }
    }
}
macro_rules! impl_float_to_float {
    ($src: ident,) => {};
    ($src: ident, $dest: ident, $($dests: ident,)*) => {
        impl_float_to_float_internal!($src, $dest);
        impl_float_to_float_internal!($dest, $src);
        impl_float_to_float!($src, $($dests,)*);
    };
}
impl_float_to_float!(f32, f64,);

#[test]
fn test_as_num() {
    // we assume that isize/usize occupy at least 32 bit (i.e. 4 byte)
    // TODO tests: improve
    assert_eq!(4i8, 4i8.as_num());
    assert_eq!(4usize, 4u16.as_num());
    assert_eq!(4i32, 4usize.as_num());
    assert_eq!(256isize.checked_as_num::<u8>(), None);
    assert_eq!(4.3.checked_as_num::<isize>(), None);
}
