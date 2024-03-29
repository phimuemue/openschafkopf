#[macro_export]
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

#[macro_export]
macro_rules! if_then_true {
    ($cond: expr, $val: expr) => {
        if $cond {
            let () = $val; // $val's type must be ()
            true
        } else {
            false
        }
    };
}


