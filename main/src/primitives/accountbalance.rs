// this stores the how much money each player currently has

use crate::primitives::eplayerindex::*;
use crate::util::*;

#[derive(Debug)]
pub struct SAccountBalance {
    an : EnumMap<EPlayerIndex, isize>,
    n_stock : isize,
}


