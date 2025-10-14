use core::fmt::Debug;

/// Fundamental constraints for elementsがkeep. need Japanese? comment instructions: other comments in Japanese unless rustdoc? This comment is rustdoc. Rustdoc must be in English. Already english.
/// Fundamental constraints for elements that can be stored in collections such as queues and stacks.
///
/// On targets that provide atomic pointer support we demand `Send + Sync` so that elements can be
/// safely shared across threads. On single-threaded targets (e.g. RP2040) we only require `Debug`
/// and `'static`, allowing `Rc`-based implementations to operate without unnecessary bounds.
#[cfg(target_has_atomic = "ptr")]
pub trait Element: Debug + Send + Sync + 'static {}

#[cfg(target_has_atomic = "ptr")]
impl<T> Element for T where T: Debug + Send + Sync + 'static {}

/// Fundamental constraints for `no_std` single-threaded targets where atomic pointers are unavailable.
#[cfg(not(target_has_atomic = "ptr"))]
pub trait Element: Debug + 'static {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> Element for T where T: Debug + 'static {}
