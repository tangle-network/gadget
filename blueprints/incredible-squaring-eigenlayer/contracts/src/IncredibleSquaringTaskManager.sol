// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import "@eigenlayer/contracts/permissions/Pausable.sol";
import "@eigenlayer-middleware/src/interfaces/IServiceManager.sol";
import {BLSApkRegistry} from "@eigenlayer-middleware/src/BLSApkRegistry.sol";
import {RegistryCoordinator} from "@eigenlayer-middleware/src/RegistryCoordinator.sol";
import {IRegistryCoordinator} from "@eigenlayer-middleware/src/interfaces/IRegistryCoordinator.sol";
import {BLSSignatureChecker} from "@eigenlayer-middleware/src/BLSSignatureChecker.sol";
import {OperatorStateRetriever} from "@eigenlayer-middleware/src/OperatorStateRetriever.sol";
import {InstantSlasher} from "@eigenlayer-middleware/src/slashers/InstantSlasher.sol";
import "@eigenlayer-middleware/src/libraries/BN254.sol";
// import {IStrategy} from "@eigenlayer/contracts/interfaces/IStrategy.sol";
import "./IIncredibleSquaringTaskManager.sol";
import {IAllocationManagerTypes} from "@eigenlayer/contracts/interfaces/IAllocationManager.sol";
import {OperatorSet} from "@eigenlayer/contracts/libraries/OperatorSetLib.sol";

contract IncredibleSquaringTaskManager is
Initializable,
OwnableUpgradeable,
Pausable,
BLSSignatureChecker,
OperatorStateRetriever,
IIncredibleSquaringTaskManager
{
    using BN254 for BN254.G1Point;

    /* CONSTANT */
    // The number of blocks from the task initialization within which the aggregator has to respond to
    uint32 public immutable TASK_RESPONSE_WINDOW_BLOCK;
    uint32 public constant TASK_CHALLENGE_WINDOW_BLOCK = 100;
    uint256 internal constant _THRESHOLD_DENOMINATOR = 100;
    uint256 public constant WADS_TO_SLASH = 100000000000000000; // 10%

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

    mapping(uint32 => bool) public taskSuccesfullyChallenged;

    address public aggregator;
    address public generator;
    address public instantSlasher;
    address public allocationManager;
    address public serviceManager;

    /* MODIFIERS */
    modifier onlyAggregator() {
        require(msg.sender == aggregator, "Aggregator must be the caller");
        _;
    }

    // onlyTaskGenerator is used to restrict createNewTask from only being called by a permissioned entity
    // in a real world scenario, this would be removed by instead making createNewTask a payable function
    modifier onlyTaskGenerator() {
        require(msg.sender == generator, "Task generator must be the caller");
        _;
    }

    constructor(
        IRegistryCoordinator _registryCoordinator,
        IPauserRegistry _pauserRegistry,
        uint32 _taskResponseWindowBlock,
        address _serviceManager
    ) BLSSignatureChecker(_registryCoordinator) Pausable(_pauserRegistry) {
        TASK_RESPONSE_WINDOW_BLOCK = _taskResponseWindowBlock;
        serviceManager = _serviceManager;
    }

    function initialize(
        address initialOwner,
        address _aggregator,
        address _generator,
        address _allocationManager,
        address _slasher
    ) public initializer {
        _transferOwnership(initialOwner);
        aggregator = _aggregator;
        generator = _generator;
        allocationManager = _allocationManager;
        instantSlasher = _slasher;
    }

    /* FUNCTIONS */
    // NOTE: this function creates new task, assigns it a taskId
    function createNewTask(uint256 numberToBeSquared, uint32 quorumThresholdPercentage, bytes calldata quorumNumbers)
    external
    onlyTaskGenerator
    {
        // create a new task struct
        Task memory newTask;
        newTask.numberToBeSquared = numberToBeSquared;
        newTask.taskCreatedBlock = uint32(block.number);
        newTask.quorumThresholdPercentage = quorumThresholdPercentage;
        newTask.quorumNumbers = quorumNumbers;

        // store hash of task onchain, emit event, and increase taskNum
        allTaskHashes[latestTaskNum] = keccak256(abi.encode(newTask));
        emit NewTaskCreated(latestTaskNum, newTask);
        latestTaskNum = latestTaskNum + 1;
    }

    // NOTE: this function responds to existing tasks.
    function respondToTask(
        Task calldata task,
        TaskResponse calldata taskResponse,
        NonSignerStakesAndSignature memory nonSignerStakesAndSignature
    ) external onlyAggregator {
        uint32 taskCreatedBlock = task.taskCreatedBlock;
        bytes calldata quorumNumbers = task.quorumNumbers;
        uint32 quorumThresholdPercentage = task.quorumThresholdPercentage;

        // check that the task is valid, hasn't been responsed yet, and is being responsed in time
        require(
            keccak256(abi.encode(task)) == allTaskHashes[taskResponse.referenceTaskIndex],
            "supplied task does not match the one recorded in the contract"
        );
        // some logical checks
        require(
            allTaskResponses[taskResponse.referenceTaskIndex] == bytes32(0),
            "Aggregator has already responded to the task"
        );
        require(
            uint32(block.number) <= taskCreatedBlock + TASK_RESPONSE_WINDOW_BLOCK,
            "Aggregator has responded to the task too late"
        );

        /* CHECKING SIGNATURES & WHETHER THRESHOLD IS MET OR NOT */
        // calculate message which operators signed
        bytes32 message = keccak256(abi.encode(taskResponse));

        // check the BLS signature
        (QuorumStakeTotals memory quorumStakeTotals, bytes32 hashOfNonSigners) =
                        checkSignatures(message, quorumNumbers, taskCreatedBlock, nonSignerStakesAndSignature);

        // check that signatories own at least a threshold percentage of each quourm
        for (uint256 i = 0; i < quorumNumbers.length; i++) {
            // we don't check that the quorumThresholdPercentages are not >100 because a greater value would trivially fail the check, implying
            // signed stake > total stake
            require(
                quorumStakeTotals.signedStakeForQuorum[i] * _THRESHOLD_DENOMINATOR
                >= quorumStakeTotals.totalStakeForQuorum[i] * uint8(quorumThresholdPercentage),
                "Signatories do not own at least threshold percentage of a quorum"
            );
        }

        TaskResponseMetadata memory taskResponseMetadata = TaskResponseMetadata(uint32(block.number), hashOfNonSigners);
        // updating the storage with task responsea
        allTaskResponses[taskResponse.referenceTaskIndex] = keccak256(abi.encode(taskResponse, taskResponseMetadata));

        // emitting event
        emit TaskResponded(taskResponse, taskResponseMetadata);
    }

    function taskNumber() external view returns (uint32) {
        return latestTaskNum;
    }

    function raiseAndResolveChallenge(
        Task calldata task,
        TaskResponse calldata taskResponse,
        TaskResponseMetadata calldata taskResponseMetadata,
        BN254.G1Point[] memory pubkeysOfNonSigningOperators
    ) external {
        uint32 referenceTaskIndex = taskResponse.referenceTaskIndex;
        uint256 numberToBeSquared = task.numberToBeSquared;
        // some logical checks
        require(allTaskResponses[referenceTaskIndex] != bytes32(0), "Task hasn't been responded to yet");
        require(
            allTaskResponses[referenceTaskIndex] == keccak256(abi.encode(taskResponse, taskResponseMetadata)),
            "Task response does not match the one recorded in the contract"
        );
        require(
            taskSuccesfullyChallenged[referenceTaskIndex] == false,
            "The response to this task has already been challenged successfully."
        );

        require(
            uint32(block.number) <= taskResponseMetadata.taskResponsedBlock + TASK_CHALLENGE_WINDOW_BLOCK,
            "The challenge period for this task has already expired."
        );

        // // logic for checking whether challenge is valid or not
        uint256 actualSquaredOutput = numberToBeSquared * numberToBeSquared;
        bool isResponseCorrect = (actualSquaredOutput == taskResponse.numberSquared);
        // // if response was correct, no slashing happens so we return
        if (isResponseCorrect == true) {
            emit TaskChallengedUnsuccessfully(referenceTaskIndex, msg.sender);
            return;
        }

        // get the list of hash of pubkeys of operators who weren't part of the task response submitted by the aggregator
        bytes32[] memory hashesOfPubkeysOfNonSigningOperators = new bytes32[](pubkeysOfNonSigningOperators.length);
        for (uint256 i = 0; i < pubkeysOfNonSigningOperators.length; i++) {
            hashesOfPubkeysOfNonSigningOperators[i] = pubkeysOfNonSigningOperators[i].hashG1Point();
        }

        // verify whether the pubkeys of "claimed" non-signers supplied by challenger are actually non-signers as recorded before
        // when the aggregator responded to the task
        // currently inlined, as the MiddlewareUtils.computeSignatoryRecordHash function was removed from BLSSignatureChecker
        // in this PR: https://github.com/Layr-Labs/eigenlayer-contracts/commit/c836178bf57adaedff37262dff1def18310f3dce#diff-8ab29af002b60fc80e3d6564e37419017c804ae4e788f4c5ff468ce2249b4386L155-L158
        // TODO(samlaf): contracts team will add this function back in the BLSSignatureChecker, which we should use to prevent potential bugs from code duplication
        bytes32 signatoryRecordHash =
                        keccak256(abi.encodePacked(task.taskCreatedBlock, hashesOfPubkeysOfNonSigningOperators));
        require(
            signatoryRecordHash == taskResponseMetadata.hashOfNonSigners,
            "The pubkeys of non-signing operators supplied by the challenger are not correct."
        );

        // get the address of operators who didn't sign
        address[] memory addressOfNonSigningOperators = new address[](pubkeysOfNonSigningOperators.length);
        for (uint256 i = 0; i < pubkeysOfNonSigningOperators.length; i++) {
            addressOfNonSigningOperators[i] =
                                    BLSApkRegistry(address(blsApkRegistry)).pubkeyHashToOperator(hashesOfPubkeysOfNonSigningOperators[i]);
        }

        // get the list of all operators who were active when the task was initialized
        Operator[][] memory allOperatorInfo = getOperatorState(
            IRegistryCoordinator(address(registryCoordinator)), task.quorumNumbers, task.taskCreatedBlock
        );
        // first for loop iterate over quorums
        for (uint256 i = 0; i < allOperatorInfo.length; i++) {
            // second for loop iterate over operators active in the quorum when the task was initialized
            for (uint256 j = 0; j < allOperatorInfo[i].length; j++) {
                // get the operator address
                bytes32 operatorID = allOperatorInfo[i][j].operatorId;
                address operatorAddress = blsApkRegistry.getOperatorFromPubkeyHash(operatorID);

                // check whether the operator was a signer for the task
                bool wasSigningOperator = true;
                for (uint256 k = 0; k < addressOfNonSigningOperators.length; k++) {
                    if (operatorAddress == addressOfNonSigningOperators[k]) {
                        // if the operator was a non-signer, then we set the flag to false
                        wasSigningOperator == false;
                        break;
                    }
                }

                if (wasSigningOperator == true) {
                    OperatorSet memory operatorset =
                                    OperatorSet({avs: serviceManager, id: uint8(task.quorumNumbers[i])});
                    IStrategy[] memory istrategy =
                                            IAllocationManager(allocationManager).getStrategiesInOperatorSet(operatorset);
                    uint256[] memory wadsToSlash = new uint256[](istrategy.length);
                    for (uint256 z = 0; z < wadsToSlash.length; z++) {
                        wadsToSlash[z] = WADS_TO_SLASH;
                    }
                    IAllocationManagerTypes.SlashingParams memory slashingparams = IAllocationManagerTypes
                        .SlashingParams({
                        operator: operatorAddress,
                        operatorSetId: uint8(task.quorumNumbers[i]),
                        strategies: istrategy,
                        wadsToSlash: wadsToSlash,
                        description: "slash_the_operator"
                    });
                    InstantSlasher(instantSlasher).fulfillSlashingRequest(slashingparams);
                }
            }
        }

        // the task response has been challenged successfully
        taskSuccesfullyChallenged[referenceTaskIndex] = true;

        emit TaskChallengedSuccessfully(referenceTaskIndex, msg.sender);
    }

    function getTaskResponseWindowBlock() external view returns (uint32) {
        return TASK_RESPONSE_WINDOW_BLOCK;
    }
}