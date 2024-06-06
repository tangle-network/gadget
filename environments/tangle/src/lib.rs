use gadget_common::prelude::TanglePalletSubmitter;
use std::sync::Arc;

pub type TangleTransactionManager = Arc<dyn TanglePalletSubmitter>;
