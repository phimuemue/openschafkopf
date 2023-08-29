pub trait TNegExt {
    fn neg_if(self, b: bool) -> Self;
}

impl<T: std::ops::Neg<Output=T>> TNegExt for T {
    fn neg_if(self, b: bool) -> Self {
        if b {
            self.neg()
        } else {
            self
        }
    }
}


