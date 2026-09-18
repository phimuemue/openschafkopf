use plain_enum::*;
use super::{assign::*, verify::*};

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

    pub fn new_with_single(e: E) -> Self {
        let mut enumset = Self::new_empty();
        verify!(enumset.insert(e));
        enumset
    }

    pub fn complement(&/*TODO? mut*/self) -> Self {
        Self::new_from_fn(|e| !self.contains(e))
    }

    pub fn minus(&/*TODO? mut*/self, rhs: &Self) -> Self
        where
            E: Copy, // TODO needed?
    {
        Self::new_from_fn(|e|
            self.contains(e) && !rhs.contains(e)
        )
    }

    pub fn intersection(&/*TODO? mut*/self, rhs: &Self) -> Self
        where
            E: Copy, // TODO needed?
    {
        Self::new_from_fn(|e|
            self.contains(e) && rhs.contains(e)
        )
    }

    pub fn iter(&self) -> impl Iterator<Item=E> + std::fmt::Debug + '_
        where
            E: Copy, // TODO needed?
    {
        E::values().filter(|e| self.contains(*e))
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
