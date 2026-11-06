//! Cœur de l'émulateur Galaxian (Namco/Midway, 1979), cycle-exact, validé contre MAME 0.287.
pub mod audio;
pub mod board;
pub mod consts;
pub mod crc;
pub mod gfx;
pub mod input;
pub mod machine;
pub mod palette;
pub mod rom;
pub mod stars;
pub mod trace;
pub mod video;

pub use board::Board;
pub use input::{InputScript, InputState};
pub use machine::Machine;
pub use rom::RomSet;
