macro_rules! make_box_clone {
    ($box_clone_trait:ident, $trait:ident) => {
        pub trait $box_clone_trait {
            fn box_clone(&self) -> Box<dyn $trait>;
        }

        impl<T: $trait + Clone + 'static> $box_clone_trait for T {
            fn box_clone(&self) -> Box<dyn $trait> {
                Box::new(self.clone())
            }
        }

        impl Clone for Box<dyn $trait> {
            fn clone(&self) -> Box<dyn $trait> {
                $box_clone_trait::box_clone(self.as_ref())
            }
        }
    };
}
