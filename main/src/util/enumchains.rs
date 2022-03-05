use plain_enum::*;
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct SEnumChains<E: TPlainEnum + TInternalEnumMapType<E, E>> {
    mapee_next: EnumMap<E, E>,
    mapee_prev: EnumMap<E, E>,
}

// TODO plain_enum: support derive(PartialEq)
impl<E: TPlainEnum + TInternalEnumMapType<E, E>> PartialEq for SEnumChains<E>
    where
        <E as TInternalEnumMapType<E, E>>::InternalEnumMapType: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.mapee_next.as_raw() == other.mapee_next.as_raw()
            && self.mapee_prev.as_raw() == other.mapee_prev.as_raw()
    }
}

#[derive(Debug, Clone)]
pub struct SRemoved<E> {
    e: E,
    e_next_old: E,
    e_prev_old: E,
}

impl<E: TPlainEnum + TInternalEnumMapType<E, E> + Copy + std::cmp::Eq + std::fmt::Debug> SEnumChains<E> {
    pub fn new() -> Self {
        let enumchains = Self {
            mapee_next: E::map_from_fn(|e| e),
            mapee_prev: E::map_from_fn(|e| e),
        };
        enumchains.assert_invariant();
        enumchains
    }

    pub fn new_from_slices(slcslce: &[&[E]]) -> Self {
        let mut enumchains = Self::new();
        for slce in slcslce {
            enumchains.chain(slce);
        }
        enumchains.assert_invariant();
        enumchains
    }

    fn assert_invariant(&self) { #[cfg(debug_assertions)] {
        for e in E::values() {
            if let Some(e_next) = self.next_no_invariant(e) {
                assert!(self.mapee_prev[e_next]==e, "{:?} -> {:?}", self, e);
            }
            if let Some(e_prev) = self.prev_no_invariant(e) {
                assert!(self.mapee_next[e_prev]==e, "{:?} -> {:?}", self, e);
            }
        }
        // TODO
    }}

    pub fn chain(&mut self, slce: &[E]) {
        for e in slce.iter() {
            assert_eq!(self.mapee_next[*e], *e);
        }
        for (e_lo, e_hi) in slce.iter().tuple_windows() {
            self.mapee_next[*e_lo] = *e_hi;
            self.mapee_prev[*e_hi] = *e_lo;
        }
        self.assert_invariant();
    }

    pub fn remove_from_chain(&mut self, e: E) -> SRemoved<E>
        where
            <E as TInternalEnumMapType<E, E>>::InternalEnumMapType: PartialEq,
    {
        #[cfg(debug_assertions)] let enumchains_clone = self.clone();
        // TODO can the following use fewer branches?
        let removed = match (self.prev(e), self.next(e)) {
            (None, None) => {
                SRemoved{e, e_prev_old:e, e_next_old:e}
            },
            (Some(e_prev_old), None) => {
                self.mapee_next[e_prev_old] = e_prev_old;
                SRemoved{e, e_prev_old, e_next_old:e}
            },
            (None, Some(e_next_old)) => {
                self.mapee_prev[e_next_old] = e_next_old;
                SRemoved{e, e_prev_old:e, e_next_old}
            },
            (Some(e_prev_old), Some(e_next_old)) => {
                assert_ne!(e_prev_old, e_next_old);
                self.mapee_next[e_prev_old] = e_next_old;
                self.mapee_prev[e_next_old] = e_prev_old;
                SRemoved{e, e_prev_old, e_next_old}
            },
        };
        self.mapee_next[e] = e;
        self.mapee_prev[e] = e;
        self.assert_invariant();
        #[cfg(debug_assertions)] // TODO why is this needed?
        debug_assert_eq!(
            enumchains_clone,
            {
                let mut enumchains_readd = self.clone();
                enumchains_readd.readd(removed.clone());
                enumchains_readd
            },
            "{:?}\n{:?}", e, removed,
        );
        removed
    }

    pub fn readd(&mut self, removed: SRemoved<E>) {
        let e = removed.e;
        assert_eq!(self.mapee_prev[e], e);
        assert_eq!(self.mapee_next[e], e);
        self.mapee_prev[e] = removed.e_prev_old;
        self.mapee_next[e] = removed.e_next_old;
        if e!=removed.e_next_old {
            self.mapee_prev[removed.e_next_old] = e;
        }
        if e!=removed.e_prev_old {
            self.mapee_next[removed.e_prev_old] = e;
        }
        self.assert_invariant();
    }

    pub fn next(&self, e: E) -> Option<E> {
        self.assert_invariant();
        self.next_no_invariant(e)
    }

    pub fn next_no_invariant(&self, e: E) -> Option<E> {
        let e_raw_next = self.mapee_next[e];
        if e_raw_next!=e { // TODO if_then_some
            Some(e_raw_next)
        } else {
            None
        }
    }

    pub fn prev(&self, e: E) -> Option<E> {
        self.assert_invariant();
        self.prev_no_invariant(e)
    }

    pub fn prev_no_invariant(&self, e: E) -> Option<E> {
        let e_raw_prev = self.mapee_prev[e];
        if e_raw_prev!=e { // TODO if_then_some
            Some(e_raw_prev)
        } else {
            None
        }
    }

    pub fn prev_while(&self, e: E, fn_pred: impl Fn(E)->bool) -> E {
        self.assert_invariant();
        assert!(fn_pred(e));
        let mut e_out = e;
        while let Some(e_prev) = self.prev(e_out) {
            if fn_pred(e_prev) {
                e_out = e_prev;
            } else {
                break;
            }
        }
        e_out
    }
}

#[cfg(test)]
plain_enum_mod!(modetestvalues, ETestValues { V0, V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11, V12, V13, });

#[test]
fn test_enumchains() {
    use ETestValues::*;
    let mut enumchains = SEnumChains::new();
    enumchains.chain(&[V0, V1, V3]);
    enumchains.chain(&[V2, V5, V10]);
    enumchains.chain(&[V8, V7, V6, V12]);
    for (e, e_prev) in [
        (V1, V0), (V3, V1),
        (V5, V2), (V10, V5),
        (V7, V8), (V6, V7), (V12, V6),
    ].into_iter() {
        assert_eq!(enumchains.prev(e), Some(e_prev));
    }
    for (e, e_prev_while) in [
        (V1, V0), (V3, V0),
        (V5, V2), (V10, V2),
        (V7, V8), (V6, V8), (V12, V8),
    ].into_iter() {
        assert_eq!(enumchains.prev_while(e, |_| true), e_prev_while);
    }
    for e in [V4, V9, V11, V13].into_iter() {
        assert_eq!(enumchains.prev(e), None);
        assert_eq!(enumchains.prev_while(e, |_| true), e);
    }
    let mut enumchains_2 = enumchains.clone();
    let removed_1 = enumchains_2.remove_from_chain(V9);
    let removed_2 = enumchains_2.remove_from_chain(V8);
    let removed_3 = enumchains_2.remove_from_chain(V6);
    let removed_4 = enumchains_2.remove_from_chain(V10);
    enumchains_2.readd(removed_4);
    enumchains_2.readd(removed_3);
    enumchains_2.readd(removed_2);
    enumchains_2.readd(removed_1);
    assert_eq!(&enumchains, &enumchains_2);
}
