// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "tnt-core/src/BlueprintServiceManagerBase.sol";
import "examples/ExampleInstance.sol";

/**
 * @title ExampleBlueprint
 * @dev This contract is an example of a service blueprint that provides a single
 * service to square a number. It demonstrates the lifecycle hooks that can be
 * implemented in a service blueprint.
 */
contract ExampleBlueprint is BlueprintServiceManagerBase {
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
     * @dev Hook for service initialization. Called when a service is initialized.
     * This hook is called after the service is approved from all of the operators.
     *
     * @param requestId The ID of the request.
     * @param serviceId The ID of the service.
     * @param owner The owner of the service.
     * @param permittedCallers  The list of permitted callers for the service.
     * @param ttl The time-to-live for the service.
     */
    function onServiceInitialized(
        uint64 requestId,
        uint64 serviceId,
        address owner,
        address[] calldata permittedCallers,
        uint64 ttl
    )
    external
    virtual
    override
    onlyFromMaster
    {
        ExampleInstance deployed = new ExampleInstance(serviceId);
        serviceInstances[serviceId] = address(deployed);
    }

    /**
     * @dev Hook for handling job result. Called when operators send the result
     * of a job execution.
     * @param serviceId The ID of the service related to the job.
     * @param job The job identifier.
     * @param jobCallId The unique ID for the job call.
     * @param operator The operator sending the result in bytes format.
     * @param inputs Inputs used for the job execution in bytes format.
     * @param outputs Outputs resulting from the job execution in bytes format.
     */
    function onJobResult(
        uint64 serviceId,
        uint8 job,
        uint64 jobCallId,
        ServiceOperators.OperatorPreferences calldata operator,
        bytes calldata inputs,
        bytes calldata outputs
    )
    external
    payable
    virtual
    override
    onlyFromMaster
    {
        if (jobCallId == 0) {
            // Decode the output
            uint256 output = abi.decode(outputs, (uint256));
            // For this example, we're just requiring the result to be 1
            bool isValid = output == 1;
            require(isValid, "Invalid result");
        }
    }
}
