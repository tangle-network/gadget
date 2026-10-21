use blueprint_client_tangle::TransactionResult;
use serde_json::json;

pub(super) fn log_tx(prefix: &str, tx: &TransactionResult, json: bool) {
    if json {
        println!(
            "{}",
            json!({
                "event": "tx_submitted",
                "action": prefix,
                "tx_hash": format!("{:#x}", tx.tx_hash),
            })
        );
        println!(
            "{}",
            json!({
                "event": "tx_confirmed",
                "action": prefix,
                "tx_hash": format!("{:#x}", tx.tx_hash),
                "block": tx.block_number,
                "gas_used": tx.gas_used,
                "success": tx.success,
            })
        );
        return;
    }

    println!("{prefix}: submitted tx_hash={:#x}", tx.tx_hash);
    if tx.success {
        println!(
            "{prefix}: confirmed block={:?} gas_used={}",
            tx.block_number, tx.gas_used
        );
    } else {
        println!(
            "{prefix} failed: tx_hash={:#x} block={:?}",
            tx.tx_hash, tx.block_number
        );
    }
}
