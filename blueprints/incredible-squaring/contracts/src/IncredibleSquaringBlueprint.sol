// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.13;

import "tnt-core/JobResultVerifier.sol";

contract IncredibleSquaringBlueprint is JobResultVerifier {
    function verify(
        uint64 serviceId,
        uint8 jobIndex,
        uint64 jobCallId,
        bytes calldata participant,
        bytes calldata inputs,
        bytes calldata outputs
    ) public override view onlyRuntime {
        require(jobIndex == 0, "Invalid job index");

        uint256 inputNumber = abi.decode(inputs, (uint256));
        uint256 outputNumber = abi.decode(outputs, (uint256));
        require(outputNumber == inputNumber * inputNumber, "Incorrect output");
    }
}

