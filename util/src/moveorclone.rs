// I expected to find something like this in Rust's stdlib (possibly in std::borrow), but the documentation did not show anything.
// It may be because, usually, functions are better off by just accepting the target type.
// However, there *may* be valid use cases for this trait, so keep it for now, and see if it proves useful.

pub trait TMoveOrClone<Target> {
    fn move_or_clone(self) -> Target;
}

impl<T> TMoveOrClone<T> for T {
    fn move_or_clone(self) -> T {
        self
    }
}

impl<T: Clone> TMoveOrClone<T> for &T {
    fn move_or_clone(self) -> T {
        self.clone()
    }
}
