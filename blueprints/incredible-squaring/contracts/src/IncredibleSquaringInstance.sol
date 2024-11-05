// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

/**
 * @title IncredibleSquaringInstance
 * @dev This contract represents an instance of the IncredibleSquaring service.
 * It is deployed when a user requests a new service instance and handles the 
 * actual squaring computation.
 */
contract IncredibleSquaringInstance {
    // Address of the blueprint that created this instance
    address public immutable blueprint;
    
    // Service ID assigned by the root chain
    uint64 public immutable serviceId;

    constructor(uint64 _serviceId) {
        blueprint = msg.sender;
        serviceId = _serviceId;
    }
}
