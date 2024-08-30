// SPDX-License-Identifier: UNLICENSE
pragma solidity >=0.8.13;

import "tnt-core/BlueprintServiceManager.sol";

/**
 * @title HelloBlueprint
 * @dev This contract is an example of a service blueprint that provides a single service.
 */
contract HelloBlueprint is BlueprintServiceManager {
    /**
     * @dev Hook for service operator registration. Called when a service operator
     * attempts to register with the blueprint.
     * @param operator The operator's details.
     * @param _registrationInputs Inputs required for registration.
     */
    function onRegister(bytes calldata operator, bytes calldata _registrationInputs)
        public
        payable
        override
        onlyFromRootChain
    {
        // Do something with the operator's details
    }

    /**
     * @dev Hook for service instance requests. Called when a user requests a service
     * instance from the blueprint.
     * @param serviceId The ID of the requested service.
     * @param operators The operators involved in the service.
     * @param _requestInputs Inputs required for the service request.
     */
    function onRequest(uint64 serviceId, bytes[] calldata operators, bytes calldata _requestInputs)
        public
        payable
        override
        onlyFromRootChain
    {
        // Do something with the service request
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
        // Do something with the job call result
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
        // Verify the job call result here
        return true;
    }

    /**
     * @dev Converts a public key to an operator address.
     * @param publicKey The public key to convert.
     * @return address The operator address.
     */
    function operatorAddressFromPublicKey(bytes calldata publicKey) internal pure returns (address) {
        return address(uint160(uint256(keccak256(publicKey))));
    }
}
