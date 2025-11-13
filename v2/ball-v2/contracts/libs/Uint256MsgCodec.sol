// SPDX-License-Identifier: MIT

pragma solidity ^0.8.22;

error InvalidMsgLength();
error InvalidStringValueLength();

library Uint256MsgCodec {
    uint16 public constant ABA_TYPE = 2;

    /// @notice Encodes a uint256 into bytes (just ABI encode it)
    function encode(uint256 _value) internal pure returns (bytes memory) {
        return abi.encode(_value);
    }

    /// @notice Decodes a uint256 from bytes
    function decode(bytes calldata _msg) internal pure returns (uint256 value) {
        if (_msg.length != 32) revert InvalidMsgLength();
        value = abi.decode(_msg[:32], (uint256));
    }

    /// @notice Encodes a uint256 with message type and return options for ABA pattern
    function encodeABA(
        uint256 _value,
        bytes calldata _returnOptions
    ) internal pure returns (bytes memory) {
        return abi.encode(_value, ABA_TYPE, _returnOptions);
    }

    /// @notice Decodes ABA message format
    /// @dev Returns (value, msgType, returnOptions)
    /// @dev If message is vanilla (32 bytes), returns msgType = 0
    function decodeABA(bytes calldata _msg) internal pure returns (
        uint256 value,
        uint16 msgType,
        bytes memory returnOptions
    ) {
        // ABA format: abi.encode(uint256, uint16, bytes)
        // Minimum size for ABA: 32 (uint256) + 32 (uint16 padded) + 32 (offset) + 32 (length) = 128 bytes
        // Vanilla format: abi.encode(uint256) = 32 bytes
        if (_msg.length == 32) {
            // Vanilla message
            value = abi.decode(_msg, (uint256));
            msgType = 0;
            returnOptions = "";
        } else {
            // ABA message
            (value, msgType, returnOptions) = abi.decode(_msg, (uint256, uint16, bytes));
        }
    }
}