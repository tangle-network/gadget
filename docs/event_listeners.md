# Event Listeners

When building a blueprint, your application may require to listen to events. Events can be of any type, and handling those events is entirely up to your discretion.

In general, when defining your job, you must register any event listeners and provide a context as such:

```rust
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(TangleEventListener, MyEventListener1, MyEventListener2, ...), // <-- Register all event listeners here
    verifier(evm = "MyVerifier")
)]
pub fn hello_event_listener(
    x: u64,
    context: MyContext, // <-- The context type must be the first additional parameter
    env: GadgetConfiguration<parking_lot::RawRwLock>,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
```

In order to make these registered event listeners to work, we must define structs that implement `EventListener`.
## Creating Event Listeners

To create an event listener, begin by defining a struct or enum which will listen to events:

```rust
use gadget_sdk::event_listener::EventListener;
use async_trait::async_trait;

/// Define a simple event listener that ticks every MS milliseconds.
pub struct Ticker<const MSEC: usize> {
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
impl<const MSEC: usize> EventListener<Instant, MyContext> for Ticker<MSEC> {
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
        tokio::time::sleep(tokio::time::Duration::from_millis(MSEC as u64 + self.additional_delay)).await;
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

## Primary Event Listeners
Primary event listeners are the most common type of event listener. They are easy to use, and require no context to work since this is done behind the scenes.

### TangleEventListener
The `TangleEventListener` is a type that listens to the Tangle network for events. This is a required type if you expect your application to use the tangle network to listen to jobs.

The `TangleEventListener` is already implemented and ready to use. Simply register it in the `job` macro, and your application
will automatically work with the Tangle network.

```rust
/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(TangleEventListener),
    verifier(evm = "IncredibleSquaringBlueprint")
)]
pub fn xsquare(
    x: u64,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2u32))
}
```

### EvmEventListener
The `EvmEventListener` is a type that listens to the Ethereum Virtual Machine (EVM) for events.

Like the `TangleEventListener`, the `EvmEventListener` is already implemented and ready to use. Simply register it in the `job` macro, and your application will automatically work with the EVM.

```rust
/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(EvmContractEventListener(
        instance = IncredibleSquaringTaskManager,
        event = IncredibleSquaringTaskManager::NewTaskCreated,
        event_converter = convert_event_to_inputs,
        callback = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerCalls::respondToTask
    )),
)]
pub fn xsquare(
    x: u64,
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
* `Event`: The event type
* `Ctx`: The context type

We can make a `PeriodicEventListener` that ticks every 5000ms to check the status of a web server using [reqwest](crates.io/crates/reqwest).

```rust
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

## Multiple Event Listeners
Arbitrarily many event listeners can be used in a single job. This is useful when you want to listen to multiple sources to handle job logic

For example, using the example of the `PeriodicEventListener` above, we can add a `TangleEventListener` to the job:

```rust
#[job(
    id = 0,
    params(x),
    result(_),
    event_listener(TangleEventListener, PeriodicEventListener::<6000, WebPoller, serde_json::Value, MyContext>), // <-- Register the event listeners here
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

In this case, both the TangleEventListener, which is listening to the Tangle network, and the PeriodicEventListener, which is polling a web server, will be used in *parallel* to listen for events.