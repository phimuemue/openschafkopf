pub fn via_out_param_init<
    T: std::borrow::BorrowMut<TBorrow>,
    TBorrow: ?Sized,
    R,
    F: FnOnce(&mut TBorrow) -> R,
>(
    mut t: T,
    f: F,
) -> (T, R) {
    let r = f(t.borrow_mut());
    (t, r)
}

pub fn via_out_param_init_result<
    T: std::borrow::BorrowMut<TBorrow>,
    TBorrow: ?Sized,
    R,
    E,
    F: FnOnce(&mut TBorrow) -> Result<R, E>,
>(
    t: T,
    f: F,
) -> Result<(T, R), E> {
    let (t_result, resr) = via_out_param_init(t, f);
    resr.map(|r| (t_result, r))
}

pub fn via_out_param<T: Default, R, F: FnOnce(&mut T) -> R>(f: F) -> (T, R) {
    via_out_param_init(Default::default(), f)
}

pub fn via_out_param_result<T: Default, R, E, F: FnOnce(&mut T) -> Result<R, E>>(
    f: F,
) -> Result<(T, R), E> {
    via_out_param_init_result(Default::default(), f)
}
