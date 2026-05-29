#[derive(Debug)]
pub struct SStringifiedError(pub String);
impl std::fmt::Display for SStringifiedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(formatter, "{:?}", self)
    }
}
//impl std::error::Error for SStringifiedError {} // TODO
pub type Error = SStringifiedError;
impl<E: std::error::Error> From<E> for SStringifiedError {
    fn from(err: E) -> SStringifiedError {
        Self(format!("{:?}", err))
    }
}

#[macro_export]
macro_rules! format_err{($($tt:tt)*) => {
    SStringifiedError(format!($($tt)*))
}}

