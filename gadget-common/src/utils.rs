use futures::Stream;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

/// A Channel Receiver that can be cloned.
///
/// On the second clone, the original channel will stop receiving new messages
/// and the new channel will start receiving any new messages after the clone.
pub struct CloneableUnboundedReceiver<T> {
    rx: Arc<tokio::sync::Mutex<UnboundedReceiver<T>>>,
    is_in_use: Arc<AtomicBool>,
}

impl<T: Clone> Clone for CloneableUnboundedReceiver<T> {
    fn clone(&self) -> Self {
        // on the clone, we switch the is_in_use flag to false
        // and we return a new channel
        self.is_in_use
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Self {
            rx: self.rx.clone(),
            is_in_use: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl<T> From<UnboundedReceiver<T>> for CloneableUnboundedReceiver<T> {
    fn from(rx: UnboundedReceiver<T>) -> Self {
        Self {
            rx: Arc::new(tokio::sync::Mutex::new(rx)),
            is_in_use: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<T> Stream for CloneableUnboundedReceiver<T> {
    type Item = T;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if !self.is_in_use.load(std::sync::atomic::Ordering::SeqCst) {
            return std::task::Poll::Ready(None);
        }
        let mut rx = match self.rx.try_lock() {
            Ok(rx) => rx,
            Err(_) => return std::task::Poll::Pending,
        };
        let rx = &mut *rx;
        tokio::pin!(rx);
        rx.poll_recv(cx)
    }
}
