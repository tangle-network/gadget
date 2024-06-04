use crate::gadget::manager::AbstractGadget;
use async_trait::async_trait;
use auto_impl::auto_impl;
use std::marker::PhantomData;
use std::sync::Arc;

/// Endows the abstract gadget with a client
#[async_trait]
pub trait GadgetWithClient<ProtocolMessage, Event, Error>: Send + Sync + 'static {
    type Client: Client<Event>;

    async fn get_next_protocol_message(&self) -> Option<ProtocolMessage>;
    async fn process_event(&self, notification: Event) -> Result<(), Error>;
    async fn process_protocol_message(&self, message: ProtocolMessage) -> Result<(), Error>;
    async fn process_error(&self, error: Error);
}

#[async_trait]
#[auto_impl(Arc)]
pub trait Client<Event>: Clone + Send + Sync {
    async fn next_event(&self) -> Option<Event>;
    async fn latest_event(&self) -> Option<Event>;
}

pub struct GeneralGadget<
    Module: GadgetWithClient<ProtocolMessage, Event, Error>,
    Event,
    ProtocolMessage,
    Error,
> {
    module: Module,
    client: Arc<Module::Client>,
    phantom_data: PhantomData<(Event, ProtocolMessage, Error)>,
}

impl<Module, Event, ProtocolMessage, Error> GeneralGadget<Module, Event, ProtocolMessage, Error>
where
    Module: GadgetWithClient<ProtocolMessage, Event, Error>,
{
    pub fn new(client: Module::Client, module: Module) -> Self {
        Self {
            module,
            client: Arc::new(client),
            phantom_data: Default::default(),
        }
    }
}

#[async_trait]
impl<Module, Event, ProtocolMessage, Error> AbstractGadget
    for GeneralGadget<Module, Event, ProtocolMessage, Error>
where
    Module: GadgetWithClient<ProtocolMessage, Event, Error>,
    Event: Send + Sync + 'static,
    ProtocolMessage: Send + Sync + 'static,
    Error: std::error::Error + Send + Sync + 'static,
{
    type Event = Event;
    type ProtocolMessage = ProtocolMessage;
    type Error = Error;

    async fn next_event(&self) -> Option<Event> {
        self.client.next_event().await
    }

    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage> {
        GadgetWithClient::get_next_protocol_message(&self.module).await
    }

    async fn on_event_received(&self, notification: Self::Event) -> Result<(), Self::Error> {
        self.module.process_event(notification).await
    }

    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error> {
        GadgetWithClient::process_protocol_message(&self.module, message).await
    }

    async fn process_error(&self, error: Self::Error) {
        GadgetWithClient::process_error(&self.module, error).await
    }
}
