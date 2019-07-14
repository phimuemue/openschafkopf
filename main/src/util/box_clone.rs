macro_rules! box_clone_require {($trait_name: ident) => {
    fn box_clone(&self) -> Box<dyn $trait_name>;
}}

macro_rules! box_clone_impl_box {($trait_name: ident) => {
    impl Clone for Box<dyn $trait_name> {
        fn clone(&self) -> Box<dyn $trait_name> {
            $trait_name::box_clone(self.as_ref())
        }
    }
}}

macro_rules! box_clone_impl_by_clone {($trait_name: ident) => {
    fn box_clone(&self) -> Box<dyn $trait_name> {
        Box::new(self.clone())
    }
}}
