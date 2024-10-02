use crate::event_listener::EventListener;
use crate::Error;
use async_trait::async_trait;
use std::marker::PhantomData;
use subxt::backend::StreamOfResults;
use subxt::blocks::Block;
use subxt::ext::sp_runtime::traits::Block as BlockT;
use subxt::OnlineClient;
use subxt_core::Config;
use tokio::sync::Mutex;

pub struct SubstrateEventListener<T: Config, Block: BlockT, Ctx> {
    client: OnlineClient<T>,
    context: Ctx,
    stream: Mutex<StreamOfResults<subxt::blocks::Block<T, OnlineClient<T>>>>,
    _pd: PhantomData<Block>,
}

#[async_trait]
pub trait SubstrateEventListenerContext<T: Config>: Clone {
    fn rpc_url(&self) -> String {
        "ws://127.0.0.1:9944".to_string()
    }

    async fn handle_event(
        &self,
        event: Block<T, OnlineClient<T>>,
        client: &OnlineClient<T>,
    ) -> Result<(), Error>;
}

#[async_trait]
impl<T: Config, Block: BlockT, Ctx: Send + Sync + SubstrateEventListenerContext<T> + 'static>
    EventListener<subxt::blocks::Block<T, OnlineClient<T>>, Ctx>
    for SubstrateEventListener<T, Block, Ctx>
{
    async fn new(context: &Ctx) -> Result<Self, Error> {
        let client = OnlineClient::from_insecure_url(context.rpc_url()).await?;
        let stream = Mutex::new(client.blocks().subscribe_finalized().await?);
        Ok(Self {
            client,
            context: context.clone(),
            stream,
            _pd: PhantomData,
        })
    }

    async fn next_event(&mut self) -> Option<subxt::blocks::Block<T, OnlineClient<T>>> {
        self.stream.lock().await.next().await.and_then(|r| r.ok())
    }

    async fn handle_event(
        &mut self,
        event: subxt::blocks::Block<T, OnlineClient<T>>,
    ) -> Result<(), Error> {
        self.context.handle_event(event, &self.client).await
    }
}
