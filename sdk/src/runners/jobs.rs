use crate::events_watcher::InitializableEventHandler;

/// A builder for blueprint jobs
pub struct JobBuilder<T>
where
    T: InitializableEventHandler + Send,
{
    pub event_handler: T,
}

impl<T> From<T> for JobBuilder<T>
where
    T: InitializableEventHandler + Send,
{
    fn from(event_handler: T) -> Self {
        Self::new(event_handler)
    }
}

impl<T> JobBuilder<T>
where
    T: InitializableEventHandler + Send,
{
    /// Create a new `JobBuilder`
    pub fn new(event_handler: T) -> Self {
        Self { event_handler }
    }
}
