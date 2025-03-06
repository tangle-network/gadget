// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.12;

import {ERC20, IERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MockERC20 is ERC20("Mock Token", "MCK") {
    function mint(address account, uint256 amount) public {
        _mint(account, amount);
    }

    /// WARNING: Vulnerable, bypasses allowance check. Do not use in production!
    function transferFrom(address from, address to, uint256 amount) public virtual override returns (bool) {
        super._transfer(from, to, amount);
        return true;
    }
}