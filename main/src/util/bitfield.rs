macro_rules! set_bits(($bitfield:expr, $bits:expr, $i_shift:expr) => {
    let bits = $bits.as_num::<u64>() << $i_shift; // TODO assert that shifting does not lose information
    debug_assert_eq!($bitfield & bits, 0); // none of the touched bits are set so far
    $bitfield |= bits;
});

