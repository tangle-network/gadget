// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {Counter} from "../src/Counter.sol";

contract CounterTest is Test {
    Counter public counter;
    bytes32 constant TEST_ID = bytes32(uint256(1));
    bytes32 constant TEST_ID_2 = bytes32(uint256(2));
    uint8 constant TEST_INDEX = 0;

    function setUp() public {
        counter = new Counter();
    }

    function test_InitialSequenceIsZero() public {
        uint256[5] memory sequence = counter.getSequence(TEST_ID);
        for(uint8 i = 0; i < 5; i++) {
            assertEq(sequence[i], 0);
        }
    }

    function test_SetNumber() public {
        counter.setNumber(TEST_ID, TEST_INDEX, 42);
        uint256[5] memory sequence = counter.getSequence(TEST_ID);
        assertEq(sequence[TEST_INDEX], 42);
    }

    function test_Increment() public {
        counter.increment(TEST_ID, TEST_INDEX);
        uint256[5] memory sequence = counter.getSequence(TEST_ID);
        assertEq(sequence[TEST_INDEX], 1);
    }

    function test_MultipleIncrements() public {
        for(uint8 i = 0; i < 5; i++) {
            counter.increment(TEST_ID, TEST_INDEX);
        }
        uint256[5] memory sequence = counter.getSequence(TEST_ID);
        assertEq(sequence[TEST_INDEX], 5);
    }

    function test_IncrementMultipleIndices() public {
        for(uint8 i = 0; i < 5; i++) {
            counter.increment(TEST_ID, i);
        }
        uint256[5] memory sequence = counter.getSequence(TEST_ID);
        for(uint8 i = 0; i < 5; i++) {
            assertEq(sequence[i], 1);
        }
    }

    function test_MultipleSequences() public {
        counter.increment(TEST_ID, 0);
        counter.increment(TEST_ID_2, 0);
        
        uint256[5] memory sequence1 = counter.getSequence(TEST_ID);
        uint256[5] memory sequence2 = counter.getSequence(TEST_ID_2);
        
        assertEq(sequence1[0], 1);
        assertEq(sequence2[0], 1);
    }

    function testFuzz_SetNumber(uint256 x) public {
        vm.assume(x < type(uint256).max);
        counter.setNumber(TEST_ID, TEST_INDEX, x);
        uint256[5] memory sequence = counter.getSequence(TEST_ID);
        assertEq(sequence[TEST_INDEX], x);
    }

    function testFuzz_SetNumberMultipleIndices(uint8 index, uint256 value) public {
        vm.assume(index < 5);
        vm.assume(value < type(uint256).max);
        counter.setNumber(TEST_ID, index, value);
        uint256[5] memory sequence = counter.getSequence(TEST_ID);
        assertEq(sequence[index], value);
    }

    function test_RevertWhenIndexOutOfBounds() public {
        vm.expectRevert("Index out of bounds");
        counter.setNumber(TEST_ID, 5, 1);
    }

    function test_RevertWhenIncrementIndexOutOfBounds() public {
        vm.expectRevert("Index out of bounds");
        counter.increment(TEST_ID, 5);
    }

    function test_EventEmission() public {
        vm.expectEmit(true, true, true, true);
        emit Counter.NumberIncremented(TEST_ID, TEST_INDEX, 0, 1);
        counter.increment(TEST_ID, TEST_INDEX);
    }

    function test_SequenceIndependence() public {
        counter.setNumber(TEST_ID, 0, 1);
        counter.setNumber(TEST_ID, 1, 2);
        counter.setNumber(TEST_ID, 2, 3);
        
        uint256[5] memory sequence = counter.getSequence(TEST_ID);
        assertEq(sequence[0], 1);
        assertEq(sequence[1], 2);
        assertEq(sequence[2], 3);
        assertEq(sequence[3], 0);
        assertEq(sequence[4], 0);
    }
}
