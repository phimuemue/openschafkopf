#![allow(
    clippy::just_underscores_and_digits,
    clippy::new_without_default,
    clippy::clone_on_copy, // TODORUST I think that some types that implement Copy should not implement it (in particular: large arrays)
)]
#![cfg_attr(not(debug_assertions), allow(
    clippy::nonminimal_bool,
    clippy::let_and_return,
))]

#[macro_use]
pub(crate) mod util;
pub mod ai;
pub mod game;
pub mod game_analysis;
pub mod player;
pub mod primitives;
pub mod rules;
