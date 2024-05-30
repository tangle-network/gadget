#!/bin/bash

# cd to the directory of this script so that this can be run from anywhere
script_path=$(
    cd "$(dirname "${BASH_SOURCE[0]}")"
    pwd -P
)

CONTRACT_PATH=$script_path/middleware
cd $CONTRACT_PATH

forge build

cp $CONTRACT_PATH/out/RegistryCoordinator.sol/RegistryCoordinator.json ../json/RegistryCoordinator.json
cp $CONTRACT_PATH/out/OperatorStateRetriever.sol/OperatorStateRetriever.json ../json/OperatorStateRetriever.json
cp $CONTRACT_PATH/out/StakeRegistry.sol/StakeRegistry.json ../json/StakeRegistry.json
cp $CONTRACT_PATH/out/BLSApkRegistry.sol/BLSApkRegistry.json ../json/BLSApkRegistry.json
cp $CONTRACT_PATH/out/IBLSSignatureChecker.sol/IBLSSignatureChecker.json ../json/IBLSSignatureChecker.json
cp $CONTRACT_PATH/out/ServiceManagerBase.sol/ServiceManagerBase.json ../json/ServiceManagerBase.json
cp $CONTRACT_PATH/out/DelegationManager.sol/DelegationManager.json ../json/DelegationManager.json
cp $CONTRACT_PATH/out/ISlasher.sol/ISlasher.json ../json/ISlasher.json
cp $CONTRACT_PATH/out/StrategyManager.sol/StrategyManager.json ../json/StrategyManager.json
cp $CONTRACT_PATH/out/EigenPod.sol/EigenPod.json ../json/EigenPod.json
cp $CONTRACT_PATH/out/EigenPodManager.sol/EigenPodManager.json ../json/EigenPodManager.json
cp $CONTRACT_PATH/out/IStrategy.sol/IStrategy.json ../json/IStrategy.json
cp $CONTRACT_PATH/out/AVSDirectory.sol/AVSDirectory.json ../json/AVSDirectory.json
cp $CONTRACT_PATH/out/IRegistryCoordinator.sol/IRegistryCoordinator.json ../json/IRegistryCoordinator.json




