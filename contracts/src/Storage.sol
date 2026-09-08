// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @title Storage
/// @notice Minimal contract used to smoke-test the Kollectyve chain's pallet-revive
///         EVM backend: one storage slot, one setter, one getter, one event.
contract Storage {
    uint256 private number;

    event NumberChanged(address indexed by, uint256 oldValue, uint256 newValue);

    constructor(uint256 initial) {
        number = initial;
        emit NumberChanged(msg.sender, 0, initial);
    }

    /// @notice Store a new value.
    function store(uint256 newNumber) public {
        uint256 old = number;
        number = newNumber;
        emit NumberChanged(msg.sender, old, newNumber);
    }

    /// @notice Read the stored value.
    function retrieve() public view returns (uint256) {
        return number;
    }

    /// @notice Increment the stored value; exercises a read-modify-write in one call.
    function increment() public returns (uint256) {
        uint256 old = number;
        number = old + 1;
        emit NumberChanged(msg.sender, old, number);
        return number;
    }
}
