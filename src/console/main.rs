#[path = "../mod.rs"]
pub mod main;

pub use main::*;

#[cfg(test)]
#[path = "../scenarios/mod.rs"]
mod scenarios;

fn main() {}
