//! Cœur de l'émulateur Galaxian (Namco/Midway, 1979), cycle-exact, validé contre MAME 0.287.

pub mod consts;

pub mod crc;

pub mod rom;

pub mod gfx;

pub mod palette;

pub mod stars;

pub mod input;

pub mod video;

pub mod audio;

pub mod board;

pub mod machine;
