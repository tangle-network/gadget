// SPDX-License-Identifier: UNLICENSE
pragma solidity >=0.8.13;

import "tnt-core/JobResultVerifier.sol";

contract HelloBlueprint is JobResultVerifier {
    function verify(
        uint64 serviceId,
        uint8 jobIndex,
        uint64 jobCallId,
        bytes calldata participant,
        bytes calldata inputs,
        bytes calldata outputs
    ) public override view onlyRuntime {
      // Implement your verification logic here
    }
}
