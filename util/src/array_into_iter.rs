pub trait TArrayIntoIter<T, const N: usize> {
    // TODORUST into_iter should work on arrays directly
    fn into_iter(self) -> std::array::IntoIter<T, N>;
}
impl<T, const N: usize> TArrayIntoIter<T, N> for [T; N] {
    fn into_iter(self) -> std::array::IntoIter<T, N> {
        std::array::IntoIter::new(self)
    }
}
