# Event Listeners

When building a blueprint, your application may require to listen to events. Events can be of any type, and handling those events is entirely up to your discretion.

## Creating Event Listeners

To create an event listener, begin by defining a struct or enum which will listen to events:

```rust
use gadget_sdk::event_listener::EventListener;
use async_trait::async_trait;

/// Define a simple event listener that ticks every MS milliseconds.
pub struct Ticker<const MS: usize> {
    additional_delay: u64,
}
```

Then, define a context for the event listener:

```rust
#[derive(Copy, Clone)]
pub struct MyContext {
    pub additional_delay: u64,
}
```

Next, implement `EventListener` for `Ticker`:

```rust
/// Implement the [`EventListener`] trait for the Ticker struct. [`EventListener`] has two type parameters:
/// 
/// - T: is the type of the event that the listener listens for. This can be anything that is Send + Sync + 'static.
/// In this case, we are using [`Instant`], which is a timestamp.
///
/// - Ctx: is the context type that the listener receives when constructed. This can be anything that is Send + Sync + 'static,
/// with the special requirement that it is the *first* listed additional parameter in the [`job`] macro (i.e., a parameter not
/// in params(...)). In this case, we are using [`MyContext`].
#[async_trait]
impl<const MS: usize> EventListener<Instant, MyContext> for Ticker<MS> {
    /// Use a reference to the context, [`MyContext`], to construct the listener
    async fn new(context: &MyContext) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(Self { additional_delay: context.additional_delay })
    }

    /// Implement the logic that looks for events. In this case, we have a simple stream
    /// that returns after MS milliseconds.
    async fn next_event(&mut self) -> Option<Instant> {
        tokio::time::sleep(tokio::time::Duration::from_millis(MS as u64 + self.additional_delay)).await;
        Some(Instant::now())
    }

    /// After next_event is called, the event gets passed here. This is where you would implement
    /// listener-specific logic.
    async fn handle_event(&mut self, _event: Instant) -> Result<(), Error> {
        Ok(())
    }
}
```

Finally, register the event listener inside the `job` macro using `event_listener`:

```rust
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(Ticker::<6000>), // <-- Register the event listener here
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn hello_event_listener(
    x: u64,
    context: MyContext, // <-- The context type must be the first additional parameter
    env: GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
```

## Wrapper Event Listeners
Various wrappers exist to simplify how common operations are performed.

### PeriodicEventListener
Some programs may only be interested in checking for events at regular intervals. In this case, the `PeriodicEventListener` can be used to simplify the process.

A `PeriodicEventListener` is a wrapper that takes 4 type parameters:

* `MSEC`: The number of milliseconds between each event check.
* `T`: The inner event listener type
* `Evt`: The event type
* `Ctx`: The context type

We can make a `PeriodicEventListener` that ticks every 5000ms to check the status of a web server

```rust
use reqwest;
use gadget_sdk::event_listener::periodic::PeriodicEventListener;

/// Define an event listener that polls a webserver
pub struct WebPoller {
    pub client: reqwest::Client,
}
```

Then, implement `EventListener` for `WebPoller`:

```rust
impl EventListener<serde_json::Value, MyContext> for WebPoller {
    /// Build the event listener. Note that this time, we don't necessarily need the context
    async fn new(_context: &MyContext) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let client = reqwest::Client::new();
        Ok(Self { client })
    }

    /// Implement the logic that polls the web server
    async fn next_event(&mut self) -> Option<serde_json::Value> {
        // Send a GET request to the JSONPlaceholder API
        let response = self.client
            .get("https://jsonplaceholder.typicode.com/todos/1")
            .send()
            .await?;

        // Check if the request was successful
        if response.status().is_success() {
            // Parse the JSON response
            let resp: serde_json::Value = response.json().await?;
            Some(resp)
        } else {
            None
        }
    }

    /// Implement any handler logic when an event is received
    async fn handle_event(&mut self, _event: serde_json::Value) -> Result<(), Error> {
        Ok(())
    }
}
```

Finally, register the event listener inside the `job` macro using `event_listener`:


```rust
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(PeriodicEventListener::<6000, WebPoller, serde_json::Value, MyContext>), // <-- Register the event listener here
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn hello_event_listener(
    x: u64,
    context: MyContext,
    env: GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
```

### SubstrateEventListener
The `SubstrateEventListener` is a wrapper that listens to finality notifications from a substrate chain. It takes 3 type parameters:

* `T`: The substrate runtime configuration
* `Block`: The block type
* `Ctx`: The context type

To begin, implement a struct:

```rust
use async_trait::async_trait;
use gadget_sdk::clients::tangle::runtime::TangleConfig;
use gadget_sdk::config::GadgetConfiguration;
use gadget_sdk::event_listener::substrate::{SubstrateEventListener, SubstrateEventListenerContext};
use gadget_sdk::ext::subxt::blocks::Block;
use gadget_sdk::ext::subxt::OnlineClient;
use gadget_sdk::{job, Error};
use std::convert::Infallible;

#[derive(Copy, Clone)]
pub struct LocalSubstrateTestnetContext;
```

Next, implement `SubstrateEventListenerContext` for `LocalSubstrateTestnetContext`:

```rust
#[async_trait]
impl SubstrateEventListenerContext<TangleConfig> for LocalSubstrateTestnetContext {
    // You can override the RPC URL by overridding the rpc_url method
    // By default, it uses ws://127.0.0.1:9944
    // fn rpc_url(&self) -> String;
    
    async fn handle_event(
        &self,
        event: Block<TangleConfig, OnlineClient<TangleConfig>>,
        _client: &OnlineClient<TangleConfig>,
    ) -> Result<(), Error> {
        gadget_sdk::info!("Received block: {:?}", event.number());
        Ok(())
    }
}
```

For convenience, define a few types:

```rust
type ExtrinsicT = gadget_sdk::tangle_subxt::subxt::ext::sp_runtime::testing::ExtrinsicWrapper<()>;
type LocalTestnetBlock =
    gadget_sdk::tangle_subxt::subxt::ext::sp_runtime::testing::Block<ExtrinsicT>;
```

Finally, register the event listener inside the `job` macro using `event_listener`:

```rust
/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(SubstrateEventListener::<TangleConfig, LocalTestnetBlock, LocalSubstrateTestnetContext>),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(
    context: LocalSubstrateTestnetContext,
    x: u64,
    env: GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
```