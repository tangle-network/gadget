use crate::sync::tracked_callback_channel::TrackedCallbackChannel;
use parity_scale_codec::{Decode, Encode};
use sp_io::TestExternalities;

pub struct MultiThreadedTestExternalities {
    pub tx: TrackedCallbackChannel<InputFunction, Vec<u8>>,
}

impl Clone for MultiThreadedTestExternalities {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

pub type InputFunction = Box<dyn FnOnce() -> Vec<u8> + 'static + Send + Sync>;

impl MultiThreadedTestExternalities {
    pub fn new(mut test_externalities: TestExternalities) -> Self {
        let (tx, mut rx) = TrackedCallbackChannel::new(1024);
        let tx_clone = tx.clone();

        std::thread::spawn(move || {
            while let Some(mut resp) = rx.blocking_recv() {
                let payload: InputFunction = resp.payload();
                let res = test_externalities.execute_with(move || payload());
                let response = resp.new(res);
                tx_clone
                    .try_reply(response)
                    .expect("Failed to send response");
            }
        });

        Self { tx }
    }

    pub fn execute_with<
        T: FnOnce() -> R + Send + Sync + 'static,
        R: Encode + Decode + Send + Sync + 'static,
    >(
        &self,
        function: T,
    ) -> R {
        let wrapped_function = move || {
            let out = function();
            out.encode()
        };
        let response = self
            .tx
            .blocking_send(Box::new(wrapped_function))
            .expect("Failed to receive response");

        R::decode(&mut &response[..]).expect("Should decode")
    }

    pub async fn execute_with_async<
        T: FnOnce() -> R + Send + Sync + 'static,
        R: Encode + Decode + Send + Sync + 'static,
    >(
        &self,
        function: T,
    ) -> R {
        let wrapped_function = move || {
            let out = function();
            out.encode()
        };
        let response = self
            .tx
            .send(Box::new(wrapped_function))
            .await
            .expect("Failed to receive response");

        R::decode(&mut &response[..]).expect("Should decode")
    }
}
