// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "tnt-core/BlueprintServiceManagerBase.sol";
import "incredible-squaring/IncredibleSquaringInstance.sol";

/**
 * @title IncredibleSquaringBlueprint
 * @dev This contract is an example of a service blueprint that provides a single
 * service to square a number. It demonstrates the lifecycle hooks that can be
 * implemented in a service blueprint.
 */
contract IncredibleSquaringBlueprint is BlueprintServiceManagerBase {
    /**
     * @dev A mapping of all service operators registered with the blueprint.
     * The key is the operator's address and the value is the operator's details.
     */
    mapping(address => bytes) public operators;
    /**
     * @dev A mapping of all service instances requested from the blueprint.
     * The key is the service ID and the value is the service operator's address.
     */
    mapping(uint64 => address) public serviceInstances;

    /**
     * @dev Hook for service instance requests. Called when a user requests a service
     * instance from the blueprint.
     * @param serviceId The ID of the requested service.
     * @param operatorsOfService The operators involved in the service in bytes array format.
     * @param requestInputs Inputs required for the service request in bytes format.
     */
    function onRequest(uint64 serviceId, bytes[] calldata operatorsOfService, bytes calldata requestInputs)
        public
        payable
        virtual
        override
        onlyFromRootChain
    {
        IncredibleSquaringInstance deployed = new IncredibleSquaringInstance(serviceId);
        serviceInstances[serviceId] = address(deployed);
    }

    /**
     * @dev Verifies the result of a job call. This function is used to validate the
     * outputs of a job execution against the expected results.
     * @param serviceId The ID of the service related to the job.
     * @param job The job identifier.
     * @param jobCallId The unique ID for the job call.
     * @param participant The participant (operator) whose result is being verified.
     * @param inputs Inputs used for the job execution.
     * @param outputs Outputs resulting from the job execution.
     */
    function onJobResult(
        uint64 serviceId,
        uint8 job,
        uint64 jobCallId,
        bytes calldata participant,
        bytes calldata inputs,
        bytes calldata outputs
    ) public payable override {
        // Decode the inputs and outputs
        uint256 input = abi.decode(inputs, (uint256));
        uint256 output = abi.decode(outputs, (uint256));
        // Check if the output is the square of the input
        bool isValid = output == input * input;
        require(isValid, "Invalid result");
    }
}
