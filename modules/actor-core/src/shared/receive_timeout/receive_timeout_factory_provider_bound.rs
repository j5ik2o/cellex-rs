#[cfg(target_has_atomic = "ptr")]
pub(crate) trait ReceiveTimeoutFactoryProviderBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> ReceiveTimeoutFactoryProviderBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub(crate) trait ReceiveTimeoutFactoryProviderBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> ReceiveTimeoutFactoryProviderBound for T {}
