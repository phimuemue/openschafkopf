pub trait TVerifiableByVerifyMacro {
    fn is_verify_true(&self) -> bool;
}

impl<T> TVerifiableByVerifyMacro for Option<T> {
    fn is_verify_true(&self) -> bool {
        self.is_some()
    }
}

impl<TOk, TErr> TVerifiableByVerifyMacro for Result<TOk, TErr> {
    fn is_verify_true(&self) -> bool {
        self.is_ok()
    }
}

macro_rules! verify {($e: expr) => {
    {
        let e = $e;
        assert!(e.is_verify_true(), stringify!($e));;
        e
    }
}}

#[test]
fn test_verify() {
    verify!(Some(4));
}
