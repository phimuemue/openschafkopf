use std::borrow::Borrow;
use crate::{
    if_then_true,
    moveorclone::TMoveOrClone,
};

pub fn assign_better<T, Src: TMoveOrClone<T>+Borrow<T>>(dst: &mut T, src: Src, fn_better: impl FnOnce(&T, &T) -> bool) -> /*TODO can/should we make return type generic (e.g. support unit or Option/Result return type)*/bool {
    if_then_true!(fn_better(src.borrow(), dst), {
        *dst = src.move_or_clone();
    })
}

pub fn assign_lt<T: Ord, Src: TMoveOrClone<T>+Borrow<T>>(dst: &mut T, src: Src) -> bool {
    assign_better(dst, src, |lhs, rhs| lhs < rhs)
}

pub fn assign_gt<T: Ord, Src: TMoveOrClone<T>+Borrow<T>>(dst: &mut T, src: Src) -> bool {
    assign_better(dst, src, |lhs, rhs| lhs > rhs)
}

pub fn assign_lt_partial_ord<T: PartialOrd, Src: TMoveOrClone<T>+Borrow<T>>(dst: &mut T, src: Src) -> bool {
    assign_better(dst, src, |lhs, rhs| lhs < rhs)
}

pub fn assign_gt_partial_ord<T: PartialOrd, Src: TMoveOrClone<T>+Borrow<T>>(dst: &mut T, src: Src) -> bool {
    assign_better(dst, src, |lhs, rhs| lhs > rhs)
}

pub fn assign_min_by_key<T, K: Ord, Src: TMoveOrClone<T>+Borrow<T>>(dst: &mut T, src: Src, mut fn_key: impl FnMut(&T) -> K) -> bool {
    assign_better(dst, src, |lhs, rhs| fn_key(lhs) < fn_key(rhs))
}

pub fn assign_max_by_key<T, K: Ord, Src: TMoveOrClone<T>+Borrow<T>>(dst: &mut T, src: Src, mut fn_key: impl FnMut(&T) -> K) -> bool {
    assign_better(dst, src, |lhs, rhs| fn_key(lhs) > fn_key(rhs))
}

pub fn assign_neq<T: Eq, Src: TMoveOrClone<T>+Borrow<T>>(dst: &mut T, src: Src) -> bool {
    assign_better(dst, src, |lhs, rhs| lhs!=rhs)
}

#[test]
fn test_assign_by_key() {
    let mut n = 0;
    assign_max_by_key(&mut n, 1, |t| *t);
    assert_eq!(n, 1);
    assign_min_by_key(&mut n, 0, |t| *t);
    assert_eq!(n, 0);
}
