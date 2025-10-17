#[cfg(target_has_atomic = "ptr")]
pub(crate) trait ReceiveTimeoutSchedulerFactoryProviderBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> ReceiveTimeoutSchedulerFactoryProviderBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
pub(crate) trait ReceiveTimeoutSchedulerFactoryProviderBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> ReceiveTimeoutSchedulerFactoryProviderBound for T {}
