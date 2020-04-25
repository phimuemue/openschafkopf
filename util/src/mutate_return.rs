#[macro_export]
macro_rules! mutate_return {
    ($f: expr) => {
        |mut t, a0| t.mutate_return($f, a0)
    };
}

pub trait TMutateReturnSelf: Sized {
    fn mutate_return<A0, F: FnMut(&mut Self, A0)>(mut self, mut f: F, a0: A0) -> Self {
        f(&mut self, a0);
        self
    }
}
impl<T: Sized> TMutateReturnSelf for T {}
