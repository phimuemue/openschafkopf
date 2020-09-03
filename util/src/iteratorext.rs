// TODORUST is_sorted et al. should be part of std:
// https://github.com/rust-lang/rfcs/pull/2351,
// https://github.com/rust-lang/rfcs/blob/master/text/2351-is-sorted.md
// https://github.com/rust-lang/rust/issues/53485
// For now, use implementation from https://github.com/rust-lang/rust/blob/b5ab524ea7b536617d8abc5507a1d97b3e60a42d/src/libcore/iter/iterator.rs
pub trait IteratorExt: itertools::Itertools {
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
        F: FnMut(&Self::Item, &Self::Item) -> Option<std::cmp::Ordering>,
    {
        let mut last = match self.next() {
            Some(e) => e,
            None => return true,
        };
        for curr in self {
            if compare(&last, &curr)
                .map(|o| o == std::cmp::Ordering::Greater)
                .unwrap_or(true)
            {
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
        K: PartialOrd,
    {
        self.is_sorted_by_unstable_name_collision(|a, b| f(a).partial_cmp(&f(b)))
    }

    // TODO itertools
    fn max_set_by_key<K: Ord>(mut self, mut fn_key: impl FnMut(&Self::Item) -> K) -> Vec<Self::Item>
    where
        Self: Sized,
        K: Ord,
    {
        self.next().map_or(vec![], |item_0| {
            self.fold(vec![item_0], |mut vecitem, item| {
                match fn_key(&vecitem[0]).cmp(&fn_key(&item)) {
                    std::cmp::Ordering::Less => vecitem = vec![item],
                    std::cmp::Ordering::Equal => vecitem.push(item),
                    std::cmp::Ordering::Greater => (),
                }
                vecitem
            })
        })
    }
}

impl<It> IteratorExt for It where It: Iterator {}
