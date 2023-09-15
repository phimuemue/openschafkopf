use std::fmt::Debug;

pub trait TVerifiableByVerifyMacro {
    type ErrDisplay<'err>: Debug
        where Self: 'err; // TODORUST? https://github.com/rust-lang/rust/issues/87479
    fn is_verify_true(&self) -> Result<(), Self::ErrDisplay<'_>>;
}

impl TVerifiableByVerifyMacro for bool {
    type ErrDisplay<'err> = bool;
    fn is_verify_true(&self) -> Result<(), Self::ErrDisplay<'_>> {
        if *self {
            Ok(())
        } else {
            Err(*self)
        }
    }
}

impl<T> TVerifiableByVerifyMacro for Option<T> {
    type ErrDisplay<'err> = &'static str where Self: 'err;
    fn is_verify_true(&self) -> Result<(), Self::ErrDisplay<'_>> {
        match self {
            None => Err("None"),
            Some(_) => Ok(()),
        }
    }
}

impl<T: TVerifiableByVerifyMacro> TVerifiableByVerifyMacro for &T {
    type ErrDisplay<'err> = T::ErrDisplay<'err> where Self: 'err;
    fn is_verify_true(&self) -> Result<(), Self::ErrDisplay<'_>> {
        T::is_verify_true(self)
    }
}

impl<TOk, TErr: Debug> TVerifiableByVerifyMacro for Result<TOk, TErr> {
    type ErrDisplay<'err> = &'err TErr where Self: 'err;
    fn is_verify_true(&self) -> Result<(), Self::ErrDisplay<'_>> {
        self.as_ref().map(|_| ())
    }
}

impl<T> TVerifiableByVerifyMacro for *const T {
    type ErrDisplay<'err> = &'static str where Self: 'err;
    fn is_verify_true(&self) -> Result<(), Self::ErrDisplay<'_>> {
        if std::ptr::null()==self {
            Err("null")
        } else {
            Ok(())
        }
    }
}

impl<T> TVerifiableByVerifyMacro for *mut T {
    type ErrDisplay<'err> = &'static str where Self: 'err;
    fn is_verify_true(&self) -> Result<(), Self::ErrDisplay<'_>> {
        if std::ptr::null_mut()==*self {
            Err("null")
        } else {
            Ok(())
        }
    }
}

#[track_caller]
pub fn verify_internal<E: TVerifiableByVerifyMacro>(e: E, str_e: &str) -> E {
    if let Err(err) = e.is_verify_true() {
        panic!("verify!({}): {:?}", str_e, err);
    }
    e
}

#[macro_export]
macro_rules! verify {($e: expr) => {{
    verify_internal($e, stringify!($e)) // TODORUST why can we not make verify_internal a closure?
}}}

#[macro_export]
macro_rules! debug_verify{($e: expr) => {
    if_dbg_else!({verify!($e)}{$e})
}}

#[macro_export]
macro_rules! unwrap {
    ($e: expr) => {
        debug_verify!($e).unwrap()
    };
}

#[macro_export]
macro_rules! verify_eq {
    ($e: expr, $e_chk: expr) => {{
        let e = $e;
        assert_eq!(e, $e_chk);
        e
    }};
}

#[macro_export]
macro_rules! debug_verify_eq {($e: expr, $e_chk: expr) => {
    if_dbg_else!({verify_eq!($e, $e_chk)}{$e})
}}

#[macro_export]
macro_rules! verify_ne {
    ($e: expr, $e_chk: expr) => {{
        let e = $e;
        assert_ne!(e, $e_chk);
        e
    }};
}

#[macro_export]
macro_rules! debug_verify_ne {($e: expr, $e_chk: expr) => {
    if_dbg_else!({verify_ne!($e, $e_chk)}{$e})
}}

#[test]
fn test_verify() {
    verify!(Some(4));
}
