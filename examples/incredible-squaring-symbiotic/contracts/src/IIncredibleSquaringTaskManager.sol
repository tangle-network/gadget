// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.13;

interface IIncredibleSquaringTaskManager {
    // EVENTS
    event NewTaskCreated(uint32 indexed taskIndex, Task task);

    event TaskResponded(TaskResponse taskResponse, uint32 indexed taskIndex);

    event TaskCompleted(uint32 indexed taskIndex);

    // STRUCTS
    struct Task {
        uint256 numberToBeSquared;
        uint32 taskCreatedBlock;
    }

    // Task response is hashed and signed by operators.
    // these signatures are aggregated and sent to the contract as response.
    struct TaskResponse {
        // Can be obtained by the operator from the event NewTaskCreated.
        uint32 referenceTaskIndex;
        // This is just the response that the operator has to compute by itself.
        uint256 numberSquared;
    }

    // FUNCTIONS
    // NOTE: this function creates new task.
    function createNewTask(
        uint256 numberToBeSquared
    ) external;

    /// @notice Returns the current 'taskNumber' for the middleware
    function taskNumber() external view returns (uint32);

    /// @notice Returns the TASK_RESPONSE_WINDOW_BLOCK
    function getTaskResponseWindowBlock() external view returns (uint32);
}
