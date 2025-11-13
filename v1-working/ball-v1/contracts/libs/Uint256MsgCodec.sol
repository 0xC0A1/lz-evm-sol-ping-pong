// SPDX-License-Identifier: MIT

pragma solidity ^0.8.22;

error InvalidMsgLength();
error InvalidStringValueLength();

library Uint256MsgCodec {
    uint8 public constant VANILLA_TYPE = 1;

    /// @notice Encodes a uint256 into bytes (just ABI encode it)
    function encode(uint256 _value) internal pure returns (bytes memory) {
        return abi.encode(_value);
    }

    /// @notice Decodes a uint256 from bytes
    function decode(bytes calldata _msg) internal pure returns (uint256 value) {
        if (_msg.length != 32) revert InvalidMsgLength();
        value = abi.decode(_msg[:32], (uint256));
    }
}