#[cfg(target_has_atomic = "ptr")]
/// Marker trait ensuring factory providers satisfy runtime sharing requirements on atomic targets.
pub trait ReceiveTimeoutSchedulerFactoryProviderBound: Send + Sync {}

#[cfg(target_has_atomic = "ptr")]
impl<T: Send + Sync> ReceiveTimeoutSchedulerFactoryProviderBound for T {}

#[cfg(not(target_has_atomic = "ptr"))]
/// Marker trait for factory providers when atomic pointer support is unavailable.
pub trait ReceiveTimeoutSchedulerFactoryProviderBound {}

#[cfg(not(target_has_atomic = "ptr"))]
impl<T> ReceiveTimeoutSchedulerFactoryProviderBound for T {}
