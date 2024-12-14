pub fn fn_cmp_to_fn_eq<T>(mut fn_cmp: impl FnMut(&T, &T)->std::cmp::Ordering) -> impl FnMut(&T, &T)->bool {
    move |lhs, rhs| std::cmp::Ordering::Equal==fn_cmp(lhs, rhs)
}

pub fn fn_cmp_to_fn_le<T>(mut fn_cmp: impl FnMut(&T, &T)->std::cmp::Ordering) -> impl FnMut(&T, &T)->bool {
    move |lhs, rhs| std::cmp::Ordering::Greater!=fn_cmp(lhs, rhs)
}
