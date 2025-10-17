#[cfg(target_has_atomic = "ptr")]
pub(crate) trait ReceiveTimeoutDriverBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> ReceiveTimeoutDriverBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub(crate) trait ReceiveTimeoutDriverBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> ReceiveTimeoutDriverBound for T {}
