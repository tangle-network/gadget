// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "tnt-core/BlueprintServiceManager.sol";

/**
 * @title IncredibleSquaringBlueprint
 * @dev This contract is an example of a service blueprint that provides a single
 * service to square a number. It demonstrates the lifecycle hooks that can be
 * implemented in a service blueprint.
 */

contract IncredibleSquaringBlueprint is BlueprintServiceManager {
    /**
     * @dev A mapping of all service operators registered with the blueprint.
     * The key is the operator's address and the value is the operator's details.
     */
    mapping(address => bytes) public operators;
    /**
     * @dev A mapping of all service instances requested from the blueprint.
     * The key is the service ID and the value is the service operator's address.
     */
    mapping(uint64 => address[]) public serviceInstances;

    /**
     * @dev Hook for service operator registration. Called when a service operator
     * attempts to register with the blueprint.
     * @param operator The operator's details.
     * @param _registrationInputs Inputs required for registration.
     */
    function onRegister(
        bytes calldata operator,
        bytes calldata _registrationInputs
    ) public payable override onlyFromRootChain {
        // compute the operator's address from the operator's public key
        address operatorAddress = operatorAddressFromPublicKey(operator);
        // store the operator's details
        operators[operatorAddress] = operator;
    }

    /**
     * @dev Hook for service instance requests. Called when a user requests a service
     * instance from the blueprint.
     * @param serviceId The ID of the requested service.
     * @param operators The operators involved in the service.
     * @param _requestInputs Inputs required for the service request.
     */
    function onRequest(
        uint64 serviceId,
        bytes[] calldata operators,
        bytes calldata _requestInputs
    ) public payable override onlyFromRootChain {
        // store the service instance request
        for (uint i = 0; i < operators.length; i++) {
            address operatorAddress = operatorAddressFromPublicKey(operators[i]);
            serviceInstances[serviceId].push(operatorAddress);
        }
    }

    /**
     * @dev Hook for handling job call results. Called when operators send the result
     * of a job execution.
     * @param serviceId The ID of the service related to the job.
     * @param job The job identifier.
     * @param _jobCallId The unique ID for the job call.
     * @param participant The participant (operator) sending the result.
     * @param _inputs Inputs used for the job execution.
     * @param _outputs Outputs resulting from the job execution.
     */
    function onJobCallResult(
        uint64 serviceId,
        uint8 job,
        uint64 _jobCallId,
        bytes calldata participant,
        bytes calldata _inputs,
        bytes calldata _outputs
    ) public virtual override onlyFromRootChain {
        // check that we have this service instance
        require(
            serviceInstances[serviceId].length > 0,
            "Service instance not found"
        );
        // check if job is zero.
        require(job == 0, "Job not found");
        // Check if the participant is a registered operator
        address operatorAddress = address(bytes20(keccak256(participant)));
        require(
            operators[operatorAddress].length > 0,
            "Operator not registered"
        );
        // Check if operator is part of the service instance
        require(
            isOperatorInServiceInstance(serviceId, operatorAddress),
            "Operator not part of service instance"
        );
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
     * @return bool Returns true if the job call result is verified successfully,
     * otherwise false.
     */
    function verifyJobCallResult(
        uint64 serviceId,
        uint8 job,
        uint64 jobCallId,
        bytes calldata participant,
        bytes calldata inputs,
        bytes calldata outputs
    ) public view virtual override onlyFromRootChain returns (bool) {
        // Someone requested to verify the result of a job call.
        // We need to check if the output is the square of the input.

        // check if job is zero.
        require(job == 0, "Job not found");
        // Check if the participant is a registered operator, so we can slash
        // their stake if they are cheating.
        address operatorAddress = operatorAddressFromPublicKey(participant);
        require(
            operators[operatorAddress].length > 0,
            "Operator not registered"
        );
        // Check if operator is part of the service instance
        require(
            isOperatorInServiceInstance(serviceId, operatorAddress),
            "Operator not part of service instance"
        );
        // Decode the inputs and outputs
        uint256 input = abi.decode(inputs, (uint256));
        uint256 output = abi.decode(outputs, (uint256));
        // Check if the output is the square of the input
        bool isValid = output == input * input;
        require(isValid, "Invalid result");

        return true;
    }

    function isOperatorInServiceInstance(
        uint64 serviceId,
        address operatorAddress
    ) public view returns (bool) {
        for (uint i = 0; i < serviceInstances[serviceId].length; i++) {
            if (serviceInstances[serviceId][i] == operatorAddress) {
                return true;
            }
        }
        return false;
    }


    function operatorAddressFromPublicKey(bytes calldata publicKey)
    public
    pure
    returns (address)
    {
        return address(uint160(uint256(keccak256(publicKey))));
    }
}

