// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

contract Counter {
    struct NumberSequence {
        uint256[5] numbers;
    }

    mapping(bytes32 => NumberSequence) sequences;

    event NumberIncremented(bytes32 indexed id, uint8 indexed index, uint256 indexed oldValue, uint256 newValue);

    function setNumber(bytes32 id, uint8 index, uint256 newNumber) public {
        require(index < 5, "Index out of bounds");
        sequences[id].numbers[index] = newNumber;
    }

    function increment(bytes32 id, uint8 index) public {
        require(index < 5, "Index out of bounds");
        uint256 oldValue = sequences[id].numbers[index];
        sequences[id].numbers[index]++;
        emit NumberIncremented(id, index, oldValue, sequences[id].numbers[index]);
    }

    function getSequence(bytes32 id) public view returns (uint256[5] memory) {
        return sequences[id].numbers;
    }
}
