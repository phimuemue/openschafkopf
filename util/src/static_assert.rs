// TODORUST static_assert not available in rust
#[macro_export]
macro_rules! static_assert{($assert_name:ident($($args:tt)*)) => {
    $assert_name!($($args)*)
}}


