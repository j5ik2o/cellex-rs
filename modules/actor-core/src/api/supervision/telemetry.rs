#![cfg_attr(not(feature = "std"), allow(unused_imports))]
mod telemetry_observation_config;
#[cfg(test)]
mod tests;

pub use telemetry_observation_config::*;
