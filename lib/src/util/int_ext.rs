// TODORUST exact_div (https://github.com/rust-lang/rust/issues/139911)
pub trait IntExt : Sized {
    fn div_exact_unstable_name_collision(self, rhs: Self) -> Option<Self>;
}

impl IntExt for isize {
    fn div_exact_unstable_name_collision(self, rhs: Self) -> Option<Self> {
        assert_ne!(rhs, 0);
        // Taken from https://doc.rust-lang.org/beta/src/core/num/int_macros.rs.html#1084
        if self % rhs != 0 {
            None
        } else {
            Some(self / rhs)
        }
    }
}
