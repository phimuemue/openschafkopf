use arrayvec::ArrayVec;
use openschafkopf_util::*;

pub trait TVecExt<T> {
    fn must_find_swap_remove(&mut self, t: &T);
}

impl<T: Eq, const N: usize> TVecExt<T> for ArrayVec<T, N> {
    fn must_find_swap_remove(&mut self, t_remove: &T) {
        let i = unwrap!(self.iter().position(|t| t==t_remove));
        self.swap_remove(i);
    }
}
