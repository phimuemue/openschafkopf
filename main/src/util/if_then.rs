macro_rules! if_then_some {
    ($cond: expr, $val: expr) => {
        if $cond {
            Some($val)
        } else {
            None
        }
    };
    (let $pattern:pat = $expr: expr, $val: expr) => {
        if let $pattern = $expr {
            Some($val)
        } else {
            None
        }
    };
}


