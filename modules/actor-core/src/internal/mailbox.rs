mod priority_mailbox_builder;
mod spawner;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
#[cfg(test)]
mod tests;

pub use priority_mailbox_builder::PriorityMailboxBuilder;
pub use spawner::PriorityMailboxSpawnerHandle;
