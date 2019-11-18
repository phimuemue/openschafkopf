pub fn via_out_param<T: Default, R, F: FnOnce(&mut T)->R>(f: F) -> (T, R) {
    let mut t = Default::default();
    let r = f(&mut t);
    (t, r)
}

pub fn via_out_param_result<T: Default, R, E, F: FnOnce(&mut T)->Result<R, E>>(f: F) -> Result<(T, R), E> {
    let (t, resr) = via_out_param(f);
    resr.map(|r| (t, r))
}

