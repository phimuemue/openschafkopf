extern crate quickcheck;

macro_rules! enum_seq_len {
    ($n: expr, $enumval: ident,) => ($n);
    ($n: expr, $enumval: ident, $($enumvals: ident,)*) => (enum_seq_len!(($n + 1), $($enumvals,)*));
}

#[macro_export]
macro_rules! plain_enum {
    ($enumname: ident {
        $($enumvals: ident,)*
    } ) => {
        #[derive(PartialEq, Eq, Debug, Copy, Clone, PartialOrd, Ord, Hash)]
        pub enum $enumname {
            $($enumvals,)*
        }

        impl $enumname {
            pub fn all_values() -> [$enumname; enum_seq_len!(1, $($enumvals,)*)] {
                [$($enumname::$enumvals,)*]
            }
        }

        impl quickcheck::Arbitrary for $enumname {
            fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> $enumname {
                *$enumname::all_values().iter()
                    .nth(
                        g.gen_range(
                            0,
                            $enumname::all_values().iter().count()
                        )
                    ).unwrap()
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
    assert_eq!(vec![ETest::E1, ETest::E2, ETest::E3], ETest::all_values().iter().cloned().collect::<Vec<_>>());
}

