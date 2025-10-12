#![cfg(feature = "new-runtime")]

//! 新ランタイム API 向けの組み込みバンドル。

pub mod embedded;

pub use embedded::EmbeddedBundle;
