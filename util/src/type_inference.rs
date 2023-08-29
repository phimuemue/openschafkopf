// TODORUST this macro should not be necessary
#[macro_export]
macro_rules! type_inference{($type:ty, $e:expr) => {
    $e as $type
}}


