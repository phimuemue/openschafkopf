extern crate quickcheck;

use std::marker::PhantomData;

macro_rules! enum_seq_len {
    ($n: expr, $enumval: ident,) => ($n);
    ($n: expr, $enumval: ident, $($enumvals: ident,)*) => (enum_seq_len!(($n + 1), $($enumvals,)*));
}

pub trait TPlainEnum : Sized {
    fn from_usize(u: usize) -> Self;
    fn ubound_usize() -> usize;
    fn values() -> SEnumIterator<Self> {
        SEnumIterator{
            m_phantom: PhantomData,
            m_i_e: 0,
        }
    }
}

#[derive(Clone)]
pub struct SEnumIterator<E> where E: TPlainEnum {
    m_phantom : PhantomData<E>,
    m_i_e : usize,
}

impl<E> Iterator for SEnumIterator<E>
    where E: TPlainEnum
{
    type Item = E;
    fn next(&mut self) -> Option<E> {
        let i_e = self.m_i_e;
        self.m_i_e = self.m_i_e + 1;
        if i_e!=E::ubound_usize() {
            Some(E::from_usize(i_e))
        } else {
            None
        }
    }
}

#[macro_export]
macro_rules! plain_enum {
    ($enumname: ident {
        $($enumvals: ident,)*
    } ) => {
        #[repr(usize)]
        #[derive(PartialEq, Eq, Debug, Copy, Clone, PartialOrd, Ord, Hash)]
        pub enum $enumname {
            $($enumvals,)*
        }

        impl TPlainEnum for $enumname {
            fn ubound_usize() -> usize {
                enum_seq_len!(1, $($enumvals,)*)
            }
            fn from_usize(u: usize) -> Self {
                use std::mem;
                unsafe{mem::transmute(u)}
            }
        }

        impl quickcheck::Arbitrary for $enumname {
            fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> $enumname {
                $enumname::from_usize(g.gen_range(0, $enumname::ubound_usize()))
            }
        }
    }
}

#[cfg(test)]
plain_enum!{ETest {
    E1, E2, E3,
}}

#[test]
fn test_plain_enum() {
    assert_eq!(3, enum_seq_len!(1, E1, E2, E3,));
    assert_eq!(3, ETest::ubound_usize());
    assert_eq!(vec![ETest::E1, ETest::E2, ETest::E3], ETest::values().collect::<Vec<_>>());
}

