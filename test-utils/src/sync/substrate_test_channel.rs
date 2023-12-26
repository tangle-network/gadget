use crate::sync::tracked_callback_channel::TrackedCallbackChannel;
use sp_io::TestExternalities;

pub struct MultiThreadedTestExternalities {
    pub tx: TrackedCallbackChannel<InputFunction, ()>,
}

impl Clone for MultiThreadedTestExternalities {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

pub type InputFunction = Box<dyn FnOnce() + 'static + Send + Sync>;

impl MultiThreadedTestExternalities {
    pub fn new(mut test_externalities: TestExternalities) -> Self {
        let (tx, mut rx) = TrackedCallbackChannel::<InputFunction, ()>::new(1024);
        let tx_clone = tx.clone();

        std::thread::spawn(move || {
            while let Some(resp) = rx.blocking_recv() {
                let response = resp.new(());
                let function = resp.payload;
                test_externalities.execute_with(function);
                tx_clone
                    .try_reply(response)
                    .expect("Failed to send response");
            }
        });

        Self { tx }
    }

    pub fn execute_with<T: FnOnce() -> R + Send + Sync + 'static, R: Send + Sync + 'static>(
        &self,
        function: T,
    ) {
        let wrapped_function = move || {
            let _ = function();
        };
        self.tx
            .blocking_send(Box::new(wrapped_function))
            .expect("Failed to receive response")
    }

    pub async fn execute_with_async<
        T: FnOnce() -> R + Send + Sync + 'static,
        R: Send + Sync + 'static,
    >(
        &self,
        function: T,
    ) {
        let wrapped_function = move || {
            let _ = function();
        };
        self.tx
            .send(Box::new(wrapped_function))
            .await
            .expect("Failed to receive response")
    }
}
