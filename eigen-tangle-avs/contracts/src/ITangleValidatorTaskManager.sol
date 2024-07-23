// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "eigenlayer-middleware/src/libraries/BN254.sol";

interface ITangleValidatorTaskManager {
    struct TangleSlashingEvent {
        bytes data;
    }

    /// NOTE: This function reports slashing events from Tangle
    function reportSlashingEvent(
        TangleSlashingEvent calldata slashingEvent
    ) external;
}
