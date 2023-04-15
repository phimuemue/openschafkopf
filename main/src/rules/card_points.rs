use crate::primitives::*;
use std::borrow::Borrow;

pub fn points_card(card: ECard) -> isize {
    // by default, we assume that we use the usual points
    match card.schlag() {
        ESchlag::S7 | ESchlag::S8 | ESchlag::S9 => 0,
        ESchlag::Unter => 2,
        ESchlag::Ober => 3,
        ESchlag::Koenig => 4,
        ESchlag::Zehn => 10,
        ESchlag::Ass => 11,
    }
}

pub fn points_stich<Stich: Borrow<SStich>>(stich: Stich) -> isize {
    stich.borrow().iter()
        .map(|(_, card)| points_card(*card))
        .sum()
}

