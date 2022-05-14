pub fn assign_better<T>(dst: &mut T, src: T, fn_better: impl FnOnce(&T, &T) -> bool) {
    if fn_better(&src, dst) {
        *dst = src;
    }
}

pub fn assign_min<T: Ord>(dst: &mut T, src: T) {
    assign_better(dst, src, |lhs, rhs| lhs < rhs)
}

pub fn assign_max<T: Ord>(dst: &mut T, src: T) {
    assign_better(dst, src, |lhs, rhs| lhs > rhs)
}

pub fn assign_min_partial_ord<T: PartialOrd>(dst: &mut T, src: T) {
    assign_better(dst, src, |lhs, rhs| lhs < rhs)
}

pub fn assign_max_partial_ord<T: PartialOrd>(dst: &mut T, src: T) {
    assign_better(dst, src, |lhs, rhs| lhs > rhs)
}

pub fn assign_min_by_key<T, K: Ord>(dst: &mut T, src: T, mut fn_key: impl FnMut(&T) -> K) {
    assign_better(dst, src, |lhs, rhs| fn_key(lhs) < fn_key(rhs))
}

pub fn assign_max_by_key<T, K: Ord>(dst: &mut T, src: T, mut fn_key: impl FnMut(&T) -> K) {
    assign_better(dst, src, |lhs, rhs| fn_key(lhs) > fn_key(rhs))
}

#[test]
fn test_assign_by_key() {
    let mut n = 0;
    assign_max_by_key(&mut n, 1, |t| *t);
    assert_eq!(n, 1);
    assign_min_by_key(&mut n, 0, |t| *t);
    assert_eq!(n, 0);
}
