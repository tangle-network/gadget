// SPDX-License-Identifier: MIT
pragma solidity 0.8.25;

import {Checkpoints} from "@openzeppelin/contracts/utils/structs/Checkpoints.sol";
import {Time} from "@openzeppelin/contracts/utils/types/Time.sol";

import {IVault} from "@symbiotic/interfaces/vault/IVault.sol";

abstract contract SimpleKeyRegistry32 {
    using Checkpoints for Checkpoints.Trace208;

    error DuplicateKey();

    mapping(address => Checkpoints.Trace208) private operatorToIdx;
    mapping(bytes32 => address) private keyToOperator;
    mapping(uint208 => bytes32) private idxToKey;
    uint208 private totalKeys;

    uint208 internal constant EMPTY_KEY_IDX = 0;

    function getOperatorByKey(bytes32 key) public view returns (address) {
        return keyToOperator[key];
    }

    function getCurrentOperatorKey(address operator) public view returns (bytes32) {
        uint208 keyIdx = operatorToIdx[operator].latest();

        if (keyIdx == EMPTY_KEY_IDX) {
            return bytes32(0);
        }

        return idxToKey[keyIdx];
    }

    function getOperatorKeyAt(address operator, uint48 timestamp) public view returns (bytes32) {
        uint208 keyIdx = operatorToIdx[operator].upperLookup(timestamp);

        if (keyIdx == EMPTY_KEY_IDX) {
            return bytes32(0);
        }

        return idxToKey[keyIdx];
    }

    function updateKey(address operator, bytes32 key) internal {
        if (keyToOperator[key] != address(0)) {
            revert DuplicateKey();
        }

        uint208 newIdx = ++totalKeys;
        idxToKey[newIdx] = key;
        operatorToIdx[operator].push(Time.timestamp(), newIdx);
        keyToOperator[key] = operator;
    }
}