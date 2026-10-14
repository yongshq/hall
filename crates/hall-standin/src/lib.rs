//! The stand-in: serves the listing (version 1) hall reads from reeve, from a
//! test reeve home's world file, so hall is built without changing reeve
//! (decision 11, ADR 0003). Never reads the live reeve home.

pub mod cli;
pub mod home;
pub mod listing;
pub mod world;
