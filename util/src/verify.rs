pub trait TVerifiableByVerifyMacro {
    fn is_verify_true(&self) -> bool;
}

impl TVerifiableByVerifyMacro for bool {
    fn is_verify_true(&self) -> bool {
        *self
    }
}

impl<T> TVerifiableByVerifyMacro for Option<T> {
    fn is_verify_true(&self) -> bool {
        self.is_some()
    }
}

impl<T: TVerifiableByVerifyMacro> TVerifiableByVerifyMacro for &T {
    fn is_verify_true(&self) -> bool {
        T::is_verify_true(self)
    }
}

impl<TOk, TErr> TVerifiableByVerifyMacro for Result<TOk, TErr> {
    fn is_verify_true(&self) -> bool {
        self.is_ok()
    }
}

pub fn verify_internal<E: TVerifiableByVerifyMacro+std::fmt::Debug>(e: E, str_e: &str) -> E {
    assert!(e.is_verify_true(), "verify!({}): {:?}", str_e, e);
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
macro_rules! verify_eq {($e: expr, $e_chk: expr) => {
    {
        let e = $e;
        assert_eq!(e, $e_chk);
        e
    }
}}

#[macro_export]
macro_rules! debug_verify_eq {($e: expr, $e_chk: expr) => {
    if_dbg_else!({verify_eq!($e, $e_chk)}{$e})
}}

#[test]
fn test_verify() {
    verify!(Some(4));
}
