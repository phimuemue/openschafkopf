// TODO can we simplify all this?
#[macro_export]
macro_rules! dbg_parameter{($t:ty) => {if_dbg_else!({$t}{()})}}
#[macro_export]
macro_rules! dbg_argument{($e:expr) => {if_dbg_else!({$e}{()})}}
