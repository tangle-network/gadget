// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {BLSApkRegistry} from "@eigenlayer-middleware/src/BLSApkRegistry.sol";
import {RegistryCoordinator} from "@eigenlayer-middleware/src/RegistryCoordinator.sol";
import {BLSSignatureChecker, IRegistryCoordinator} from "@eigenlayer-middleware/src/BLSSignatureChecker.sol";

/// @title TaskManager Contract
/// @dev This contract is responsible for managing tasks, responses, and interactions between the aggregator and task generator.
abstract contract TaskManager is BLSSignatureChecker, OwnableUpgradeable {
    event NewTaskCreated(uint32 indexed taskIndex, Task task);
    event TaskResponded(TaskResponse taskResponse, TaskResponseMetadata taskResponseMetadata);

    /// @notice Emitted when the aggregator address is updated.
    /// @param oldAggregator The previous aggregator address.
    /// @param newAggregator The new aggregator address.
    event AggregatorUpdated(address indexed oldAggregator, address indexed newAggregator);

    /// @notice Emitted when the generator address is updated.
    /// @param oldGenerator The previous generator address.
    /// @param newGenerator The new generator address.
    event GeneratorUpdated(address indexed oldGenerator, address indexed newGenerator);

    error AlreadySet();
    error NoOngoingDeployment();
    error ZeroAddress();
    error ZeroValue();

    // STRUCTS

    /// @dev Defines the structure of a task.
    struct Task {
        uint32 taskCreatedBlock;
        uint32 quorumThresholdPercentage;
        bytes message;
        bytes quorumNumbers;
    }

    // Task response is hashed and signed by operators.
    // these signatures are aggregated and sent to the contract as response.
    struct TaskResponse {
        // Can be obtained by the operator from the event NewTaskCreated.
        uint32 referenceTaskIndex;
        // This is just the response that the operator has to compute by itself.
        bytes message;
    }

    // Extra information related to taskResponse, which is filled inside the contract.
    // It thus cannot be signed by operators, so we keep it in a separate struct than TaskResponse
    // This metadata is needed by the challenger, so we emit it in the TaskResponded event
    struct TaskResponseMetadata {
        uint32 taskResponsedBlock;
        bytes32 hashOfNonSigners;
    }

    /* CONSTANT */
    uint256 internal constant _THRESHOLD_DENOMINATOR = 100;

    // The number of blocks from the task initialization within which the aggregator has to respond to
    uint32 public immutable TASK_RESPONSE_WINDOW_BLOCK;

    /* STORAGE */
    // The latest task index
    uint32 public latestTaskNum;

    // mapping of task indices to all tasks hashes
    // when a task is created, task hash is stored here,
    // and responses need to pass the actual task,
    // which is hashed onchain and checked against this mapping
    mapping(uint32 => bytes32) public allTaskHashes;

    // mapping of task indices to hash of abi.encode(taskResponse, taskResponseMetadata)
    mapping(uint32 => bytes32) public allTaskResponses;

    address public aggregator;
    address public generator;

    // storage gap for upgradeability
    // slither-disable-next-line shadowing-state
    uint256[45] private __GAP;

    /* MODIFIERS */

    /// @dev Modifier to allow only the aggregator to call certain functions.
    modifier onlyAggregator() {
        require(msg.sender == aggregator, "Aggregator must be the caller");
        _;
    }

    /// @dev Modifier to allow only the task generator to create new tasks.
    modifier onlyTaskGenerator() {
        require(msg.sender == generator, "Task generator must be the caller");
        _;
    }

    /// @notice Constructor to initialize the contract.
    /// @param _registryCoordinator Address of the registry coordinator.
    /// @param _taskResponseWindowBlock Number of blocks within which the aggregator has to respond to a task.
    constructor(IRegistryCoordinator _registryCoordinator, uint32 _taskResponseWindowBlock)
        BLSSignatureChecker(_registryCoordinator)
    {
        TASK_RESPONSE_WINDOW_BLOCK = _taskResponseWindowBlock;
    }

    /// @notice Initializes the contract with aggregator and generator addresses and transfers ownership.
    /// @param _aggregator Address of the aggregator.
    /// @param _generator Address of the task generator.
    /// @param initialOwner Address of the initial owner.
    function __TaskManager_init(address _aggregator, address _generator, address initialOwner)
        public
        onlyInitializing
    {
        __Ownable_init();
        transferOwnership(initialOwner);
        _setAggregator(_aggregator);
        _setGenerator(_generator);
    }

    /// @notice Sets a new aggregator address.
    /// @dev Only callable by the contract owner.
    /// @param newAggregator Address of the new aggregator.
    function setAggregator(address newAggregator) external onlyOwner {
        _setAggregator(newAggregator);
    }

    /// @notice Sets a new generator address.
    /// @dev Only callable by the contract owner.
    /// @param newGenerator Address of the new task generator.
    function setGenerator(address newGenerator) external onlyOwner {
        _setGenerator(newGenerator);
    }

    /* FUNCTIONS */

    /// @notice Creates a new task and assigns it a taskId.
    /// @param message Message payload of the task.
    /// @param quorumThresholdPercentage Minimum percentage of quorum required.
    /// @param quorumNumbers Numbers representing the quorum.
    /// @dev Only callable by the task generator.
    function _createNewTask(bytes memory message, uint32 quorumThresholdPercentage, bytes calldata quorumNumbers)
        internal
        onlyTaskGenerator
    {
        Task memory newTask;
        newTask.message = message;
        newTask.taskCreatedBlock = uint32(block.number);
        newTask.quorumThresholdPercentage = quorumThresholdPercentage;
        newTask.quorumNumbers = quorumNumbers;

        allTaskHashes[latestTaskNum] = keccak256(abi.encode(newTask));
        emit NewTaskCreated(latestTaskNum, newTask);
        latestTaskNum++;
    }

    /// @notice Responds to an existing task.
    /// @param task The task being responded to.
    /// @param taskResponse The response data to the task.
    /// @param nonSignerStakesAndSignature Signature and stakes of non-signers for verification.
    /// @dev Only callable by the aggregator.
    function _respondToTask(
        Task calldata task,
        TaskResponse calldata taskResponse,
        NonSignerStakesAndSignature memory nonSignerStakesAndSignature
    ) internal onlyAggregator {
        uint32 taskCreatedBlock = task.taskCreatedBlock;
        bytes calldata quorumNumbers = task.quorumNumbers;
        uint32 quorumThresholdPercentage = task.quorumThresholdPercentage;

        require(
            keccak256(abi.encode(task)) == allTaskHashes[taskResponse.referenceTaskIndex],
            "Supplied task does not match the one recorded in the contract"
        );
        require(
            allTaskResponses[taskResponse.referenceTaskIndex] == bytes32(0),
            "Aggregator has already responded to the task"
        );
        require(
            uint32(block.number) <= taskCreatedBlock + TASK_RESPONSE_WINDOW_BLOCK, "Aggregator has responded too late"
        );

        bytes32 message = keccak256(abi.encode(taskResponse));

        (QuorumStakeTotals memory quorumStakeTotals, bytes32 hashOfNonSigners) =
            checkSignatures(message, quorumNumbers, taskCreatedBlock, nonSignerStakesAndSignature);

        for (uint256 i = 0; i < quorumNumbers.length; i++) {
            require(
                quorumStakeTotals.signedStakeForQuorum[i] * _THRESHOLD_DENOMINATOR
                    >= quorumStakeTotals.totalStakeForQuorum[i] * uint8(quorumThresholdPercentage),
                "Signatories do not own at least threshold percentage of a quorum"
            );
        }

        TaskResponseMetadata memory taskResponseMetadata = TaskResponseMetadata(uint32(block.number), hashOfNonSigners);
        allTaskResponses[taskResponse.referenceTaskIndex] = keccak256(abi.encode(taskResponse, taskResponseMetadata));

        emit TaskResponded(taskResponse, taskResponseMetadata);
    }

    /* INTERNAL FUNCTIONS */

    /// @dev Internal function to set a new task generator.
    /// @param newGenerator Address of the new generator.
    function _setGenerator(address newGenerator) internal {
        require(newGenerator != address(0), "Generator cannot be zero address");
        address oldGenerator = generator;
        generator = newGenerator;
        emit GeneratorUpdated(oldGenerator, newGenerator);
    }

    /// @dev Internal function to set a new aggregator.
    /// @param newAggregator Address of the new aggregator.
    function _setAggregator(address newAggregator) internal {
        require(newAggregator != address(0), "Aggregator cannot be zero address");
        address oldAggregator = aggregator;
        aggregator = newAggregator;
        emit AggregatorUpdated(oldAggregator, newAggregator);
    }
}
