use crate::gadget::manager::AbstractGadget;
use async_trait::async_trait;
use auto_impl::auto_impl;
use std::marker::PhantomData;
use std::sync::Arc;

/// Endows the abstract gadget with a client
#[async_trait]
pub trait GadgetWithClient<AbstractGadgetT: AbstractGadget>: Send + Sync + Sized {
    type Client: Client<AbstractGadgetT>;

    async fn get_next_protocol_message(&self) -> Option<AbstractGadgetT::ProtocolMessage>;
    async fn process_event(
        &self,
        notification: AbstractGadgetT::Event,
    ) -> Result<(), AbstractGadgetT::Error>;
    async fn process_protocol_message(
        &self,
        message: AbstractGadgetT::ProtocolMessage,
    ) -> Result<(), AbstractGadgetT::Error>;
    async fn process_error(&self, error: AbstractGadgetT::Error);
    fn client(&self) -> &Self::Client;
}

#[async_trait]
#[auto_impl(Arc)]
pub trait Client<AbstractGadgetT>: Clone + Send + Sync where AbstractGadgetT: AbstractGadget {
    async fn next_event(&self) -> Option<AbstractGadgetT::Event>;
    async fn latest_event(&self) -> Option<AbstractGadgetT::Event>;
}


pub struct GeneralGadget<Module: GadgetWithClient<AbstractGadgetT>, Event, ProtocolMessage, Error, AbstractGadgetT: AbstractGadget> {
    module: Module,
    client: Arc<Module::Client>,
    phantom_data: PhantomData<(Event, ProtocolMessage, Error, AbstractGadgetT)>
}

impl<Module, Event, ProtocolMessage, Error, AbstractGadgetT: AbstractGadget> GeneralGadget<Module, Event, ProtocolMessage, Error, AbstractGadgetT>
    where
        Module: GadgetWithClient<AbstractGadgetT>,
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
impl<Module, Event, ProtocolMessage, Error, AbstractGadgetT: AbstractGadget> AbstractGadget for GeneralGadget<Module, Event, ProtocolMessage, Error, AbstractGadgetT>
where
    Module: GadgetWithClient<AbstractGadgetT>,
    Event: Send + Sync,
    ProtocolMessage: Send + Sync,
    Error: std::error::Error + Send + Sync,
{
    type Event = Event;
    type ProtocolMessage = ProtocolMessage;
    type Error = Error;

    async fn next_event(&self) -> Option<Event> 
        where Module: GadgetWithClient<AbstractGadgetT>,{
        self.client.next_event().await
    }

    async fn get_next_protocol_message(&self) -> Option<AbstractGadgetT::ProtocolMessage> {
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
