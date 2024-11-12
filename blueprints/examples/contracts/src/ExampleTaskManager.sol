// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.13;

contract ExampleTaskManager {
    // Define the Task struct
    struct Task {
        address targetAddress;
        uint32 taskCreatedBlock;
        // Additional fields can be added as needed
    }

    // Event to be emitted when a new task is created
    event NewTaskCreated(uint32 indexed taskIndex, Task task);

    // Mapping to store tasks
    mapping(uint32 => Task) public tasks;
    uint32 public taskCounter;

    constructor() {
        taskCounter = 0;
    }

    // Function to create a new task
    function createTask(address targetAddress) external {
        Task memory newTask = Task({
            targetAddress: targetAddress,
            taskCreatedBlock: uint32(block.number)
        });

        tasks[taskCounter] = newTask;
        emit NewTaskCreated(taskCounter, newTask);

        taskCounter++;
    }

    // Function to get task details
    function getTask(uint32 taskIndex) external view returns (Task memory) {
        return tasks[taskIndex];
    }
}