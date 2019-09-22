pub fn assign_better<T>(dst: &mut T, src: T, fn_better: impl FnOnce(&T, &T)->bool) {
    if fn_better(&src, dst) {
        *dst = src;
    }
}

pub fn assign_min<T: Ord>(dst: &mut T, src: T) {
    assign_better(dst, src, |lhs, rhs| lhs<rhs)
}

