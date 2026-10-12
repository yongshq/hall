//! The stand-in: serves the draft listing hall needs from reeve, read from a
//! test reeve home's world file, so hall is built without changing reeve
//! (decision 11, ADR 0003). Never reads the live reeve home.

pub mod cli;
pub mod home;
pub mod listing;
pub mod world;
