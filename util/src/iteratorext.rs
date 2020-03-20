// TODORUST is_sorted et al. should be part of std:
// https://github.com/rust-lang/rfcs/pull/2351,
// https://github.com/rust-lang/rfcs/blob/master/text/2351-is-sorted.md
// https://github.com/rust-lang/rust/issues/53485
// For now, use implementation from https://github.com/rust-lang/rust/blob/b5ab524ea7b536617d8abc5507a1d97b3e60a42d/src/libcore/iter/iterator.rs
pub trait IteratorExt : itertools::Itertools {
    fn is_sorted_unstable_name_collision(self) -> bool
        where
            Self: Sized,
            Self::Item: PartialOrd,
    {
        self.is_sorted_by_unstable_name_collision(|a, b| a.partial_cmp(b))
    }

    fn is_sorted_by_unstable_name_collision<F>(mut self, mut compare: F) -> bool
        where
            Self: Sized,
            F: FnMut(&Self::Item, &Self::Item) -> Option<std::cmp::Ordering>
    {
        let mut last = match self.next() {
            Some(e) => e,
            None => return true,
        };
        for curr in self {
            if compare(&last, &curr).map(|o| o == std::cmp::Ordering::Greater).unwrap_or(true) {
                return false;
            }
            last = curr;
        }
        true
    }

    fn is_sorted_by_key_unstable_name_collision<F, K>(self, mut f: F) -> bool
        where
            Self: Sized,
            F: FnMut(&Self::Item) -> K,
            K: PartialOrd
    {
        self.is_sorted_by_unstable_name_collision(|a, b| f(a).partial_cmp(&f(b)))
    }
}

impl<It> IteratorExt for It where It: Iterator {}


