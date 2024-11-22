macro_rules! forward_to_field{
    (self.$field:ident,) => {};
    (
        self.$field:ident,
        $visibility:vis fn $fn_name:ident(/*TODO? other self types*/&self $(, $ident_arg:ident: $ty_arg:ty)*$(,)?) $(-> $ty_res:ty)?;
        $($tt:tt)*
    ) => {
        $visibility fn $fn_name(&self, $($ident_arg: $ty_arg,)*) $(-> $ty_res)? {
            self.$field.$fn_name($($ident_arg,)*)
        }
        forward_to_field!(self.$field, $($tt)*);
    };
}


