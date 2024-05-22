pub trait OptionExt<T> {
    fn insert_or_fold(&mut self, t: T, fn_accumulate: impl FnOnce(&mut T, T));
}

impl<T> OptionExt<T> for Option<T> {
    fn insert_or_fold(&mut self, t: T, fn_accumulate: impl FnOnce(&mut T, T)) {
        match self {
            None => *self = Some(t),
            Some(ref mut t_present) => fn_accumulate(t_present, t),
        };
    }
}
