use futures::stream::FuturesUnordered;
use futures::StreamExt;
use gadget_common::prelude::DebugLogger;
use std::net::ToSocketAddrs;

// Start by binding to our addr with a tokio TcpListener, then, try connecting to all other nodes iteratively with a timeout and retry
// until a connection is established. Once a connection with all nodes occurs, we can assume all nodes have reached the same state.
pub async fn sync_local_nodes<T: ToSocketAddrs, R: ToSocketAddrs>(
    our_addr: T,
    other_addrs: Vec<T>,
    logger: &DebugLogger,
) {
    let our_addr = our_addr.to_socket_addrs().unwrap().next().unwrap();
    let other_addrs = other_addrs
        .iter()
        .map(|addr| addr.to_socket_addrs().unwrap().next().unwrap())
        .collect::<Vec<_>>();

    assert!(
        !other_addrs.contains(&our_addr),
        "Our address should not be in the list of other addresses"
    );

    if other_addrs.is_empty() {
        return;
    }

    let listener = tokio::net::TcpListener::bind(our_addr).await.unwrap();
    let count = other_addrs.len();
    logger.trace(format!(
        "Bound to address: {our_addr}, waiting for {count} nodes to connect"
    ));

    let rx_task = tokio::spawn(async move {
        let mut accept_count = 0;
        loop {
            let _ = listener.accept().await;
            accept_count += 1;

            if accept_count == count {
                return ();
            }
        }
    });

    // Now, try connecting to each of the nodes in parallel, performing a retry with a timeout
    let mut tasks = FuturesUnordered::new();
    for addr in other_addrs {
        let logger = logger.clone();
        tasks.push(tokio::spawn(async move {
            loop {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    tokio::net::TcpStream::connect(addr),
                )
                .await
                {
                    Ok(Ok(_)) => {
                        logger.trace(format!("Connected to node {}", addr));
                        return ();
                    }
                    _ => {
                        logger.info(format!("Waiting for node {} to connect to us", addr));
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }));
    }

    while let Some(res) = tasks.next().await {
        res.unwrap();
    }

    rx_task.await.unwrap();
}

// Create unit tests for this sync method
#[cfg(test)]
mod tests {
    use crate::setup_log;
    use futures::stream::FuturesUnordered;
    use futures::StreamExt;
    use gadget_common::prelude::DebugLogger;
    use std::future::Future;
    use std::net::SocketAddr;
    use std::pin::Pin;

    fn find_free_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        addr.port()
    }

    #[tokio::test]
    async fn test_sync_local_nodes() {
        const COUNT: usize = 10;
        setup_log();

        let futures = FuturesUnordered::new();
        let mut addrs = vec![];
        let mut loggers = vec![];

        for _ in 0..COUNT {
            let port = find_free_port();
            let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
            let logger = DebugLogger::from(format!("Node {addr}"));
            addrs.push(addr);
            loggers.push(logger);
        }

        // Now, create and run all futures concurrently
        for i in 0..COUNT {
            let our_addr = addrs[i];
            let other_addrs = addrs
                .iter()
                .filter(|&addr| *addr != our_addr)
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(other_addrs.len(), COUNT - 1);
            let logger = loggers[i].clone();
            let task = async move {
                super::sync_local_nodes::<SocketAddr, SocketAddr>(our_addr, other_addrs, &logger)
                    .await;
            };
            futures.push(Box::pin(task) as Pin<Box<dyn Future<Output = ()>>>);
        }

        futures.collect::<()>().await;
    }
}
