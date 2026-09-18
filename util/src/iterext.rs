#[derive(Debug)]
pub enum ExactlyOneError<I: Iterator> {
    Empty,
    MoreThanOne([I::Item; 2], I),
}

pub trait IterExt : Iterator + Sized {
    // Simpler interface of exactly_one // TODO could this be upstreamed to itertools?
    fn exactly_one_2(mut self) -> Result<Self::Item, ExactlyOneError<Self>> {
        match self.next() {
            None => Err(ExactlyOneError::Empty),
                 Some(first) => match self.next() {
                     None => Ok(first),
                     Some(second) => Err(ExactlyOneError::MoreThanOne([first, second], self)),
                 }
        }
    }
}
impl<I: Iterator> IterExt for I {}
