use crate::gadget::manager::AbstractGadget;
use async_trait::async_trait;
use auto_impl::auto_impl;
use std::marker::PhantomData;
use std::sync::Arc;

/// Endows the abstract gadget with a client
#[async_trait]
pub trait GadgetWithClient: Send + Sync + AbstractGadget {
    type Client: Client;

    async fn get_next_protocol_message(&self) -> Option<<Self as AbstractGadget>::ProtocolMessage>;
    async fn process_event(
        &self,
        notification: <Self as AbstractGadget>::Event,
    ) -> Result<(), <Self as AbstractGadget>::Error>;
    async fn process_protocol_message(
        &self,
        message: <Self as AbstractGadget>::ProtocolMessage,
    ) -> Result<(), Self::Error>;
    async fn process_error(&self, error: <Self as AbstractGadget>::Error);
    fn client(&self) -> &Self::Client;
}

#[async_trait]
#[auto_impl(Arc)]
pub trait Client: Clone + Send + Sync {
    type Event: Send;
    async fn next_event(&self) -> Option<Self::Event>;
    async fn latest_event(&self) -> Option<Self::Event>;
}


pub struct GeneralGadget<Module: GadgetWithClient, Event, ProtocolMessage, Error> {
    module: Module,
    client: Arc<Module::Client>,
    phantom_data: PhantomData<(Event, ProtocolMessage, Error)>
}

impl<Module, Event, ProtocolMessage, Error> GeneralGadget<Module, Event, ProtocolMessage, Error>
    where
        Module: GadgetWithClient,
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
impl<Module, Event, ProtocolMessage, Error> AbstractGadget for GeneralGadget<Module, Event, ProtocolMessage, Error>
where
    Module: GadgetWithClient<Event = Event, Error = Error, ProtocolMessage = ProtocolMessage>,
    Event: Send + Sync,
    ProtocolMessage: Send + Sync,
    Error: std::error::Error + Send + Sync,
{
    type Event = Event;
    type ProtocolMessage = ProtocolMessage;
    type Error = Error;

    async fn next_event(&self) -> Option<Event> {
        self.client.next_event().await
    }

    async fn get_next_protocol_message(&self) -> Option<<Module as AbstractGadget>::ProtocolMessage> {
        GadgetWithClient::get_next_protocol_message(&self.module).await
    }

    async fn on_event_received(
        &self,
        notification: <Module as AbstractGadget>::Event,
    ) -> Result<(), <Module as AbstractGadget>::Error> {
        self.module
            .process_event(notification)
            .await
    }

    async fn process_protocol_message(
        &self,
        message: <Module as AbstractGadget>::ProtocolMessage,
    ) -> Result<(), Self::Error> {
        GadgetWithClient::process_protocol_message(&self.module, message).await
    }

    async fn process_error(&self, error: <Module as AbstractGadget>::Error) {
        GadgetWithClient::process_error(&self.module, error).await
    }
}
