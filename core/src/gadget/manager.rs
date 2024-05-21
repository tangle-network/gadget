use async_trait::async_trait;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;

pub struct GadgetManager<'a> {
    gadget: Pin<Box<dyn Future<Output = Result<(), GadgetError>> + Send + 'a>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GadgetError {
    FinalityNotificationStreamEnded,
    ProtocolMessageStreamEnded,
}

impl Display for GadgetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Error for GadgetError {}

#[async_trait]
pub trait AbstractGadget: Send + Sync {
    type Event: Send;
    type ProtocolMessage: Send;
    type Error: Error + Send;

    async fn next_event(&self) -> Option<Self::Event>;
    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage>;

    async fn on_event_received(
        &self,
        notification: Self::Event,
    ) -> Result<(), Self::Error>;
    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error>;

    async fn process_error(&self, error: Self::Error);
}

impl<'a> GadgetManager<'a> {
    pub fn new<T: AbstractGadget + 'a>(gadget: T) -> Self {
        let gadget_task = async move {
            let gadget = &gadget;

            let finality_notification_task = async move {
                loop {
                    if let Some(notification) = gadget.next_event().await {
                        if let Err(err) = gadget.on_event_received(notification).await {
                            gadget.process_error(err).await;
                        }
                    } else {
                        return Err(GadgetError::FinalityNotificationStreamEnded);
                    }
                }
            };

            let protocol_message_task = async move {
                loop {
                    if let Some(message) = gadget.get_next_protocol_message().await {
                        if let Err(err) = gadget.process_protocol_message(message).await {
                            gadget.process_error(err).await;
                        }
                    } else {
                        return Err(GadgetError::ProtocolMessageStreamEnded);
                    }
                }
            };

            tokio::select! {
                res0 = finality_notification_task => res0,
                res1 = protocol_message_task => res1
            }
        };

        Self {
            gadget: Box::pin(gadget_task),
        }
    }
}

impl Future for GadgetManager<'_> {
    type Output = Result<(), GadgetError>;
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.gadget.as_mut().poll(cx)
    }
}

impl<'a, T: AbstractGadget + 'a> From<T> for GadgetManager<'a> {
    fn from(gadget: T) -> Self {
        Self::new(gadget)
    }
}
