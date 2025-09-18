use plain_enum::*;
use super::assign::*;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct EnumSet<E: PlainEnum>(EnumMap<E, bool>)
    where
        E::EnumMapArray<bool>: Eq,
;

impl<E: PlainEnum> EnumSet<E>
    where
        E::EnumMapArray<bool>: Eq,
{
    pub fn new_empty() -> Self {
        Self(E::map_from_fn(|_e| false))
    }

    pub fn new_from_fn(fn_contained: impl FnMut(E)->bool) -> Self {
        Self(E::map_from_fn(fn_contained))
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|b| !b)
    }

    pub fn is_full(&self) -> bool {
        self.0.iter().all(|b| *b)
    }

    pub fn contains(&self, e: E) -> bool {
        self.0[e]
    }

    pub fn insert(&mut self, e: E) -> bool {
        assign_neq(&mut self.0[e], true)
    }
}
