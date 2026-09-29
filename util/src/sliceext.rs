#[cfg(debug_assertions)]
use crate::verify::verify_internal;
use itertools::Itertools;

pub trait SliceExt {
    type Item;
    fn chunk_by_key<'slf, K: Eq + std::fmt::Debug>( // TODORUST
        &'slf self,
        fn_key: impl FnMut(&Self::Item) -> K + 'slf + Copy,
    ) -> impl DoubleEndedIterator<Item=(K, &'slf [Self::Item])>
        where Self::Item: 'slf;
}
impl<T> SliceExt for [T] {
    type Item = T;
    fn chunk_by_key<'slf, K: Eq + std::fmt::Debug>(
        &'slf self,
        mut fn_key: impl FnMut(&Self::Item) -> K + 'slf + Copy,
    ) -> impl DoubleEndedIterator<Item=(K, &'slf [Self::Item])>
        where Self::Item: 'slf
    {
        self
            .chunk_by(move |lhs, rhs| fn_key(lhs)==fn_key(rhs))
            .map(move |slcitem| (
                unwrap!(slcitem.iter().map(fn_key).all_equal_value()), // TODO Make this efficient, avoid re-evaluating fn_key.
                slcitem,
            ))
    }
}
