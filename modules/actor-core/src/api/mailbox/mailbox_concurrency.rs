/// Marker trait describing the synchronization requirements for a mailbox factory.
pub trait MailboxConcurrency: Copy + 'static {}