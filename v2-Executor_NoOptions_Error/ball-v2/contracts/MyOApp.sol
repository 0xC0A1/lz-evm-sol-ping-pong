// SPDX-License-Identifier: MIT

pragma solidity ^0.8.22;

import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { OApp, MessagingFee, Origin } from "@layerzerolabs/oapp-evm/contracts/oapp/OApp.sol";
import { MessagingReceipt } from "@layerzerolabs/oapp-evm/contracts/oapp/OAppSender.sol";
import { OAppOptionsType3 } from "@layerzerolabs/oapp-evm/contracts/oapp/libs/OAppOptionsType3.sol";
import { OptionsBuilder } from "@layerzerolabs/oapp-evm/contracts/oapp/libs/OptionsBuilder.sol";
import { Uint256MsgCodec } from "./libs/Uint256MsgCodec.sol";

contract MyOApp is OApp, OAppOptionsType3 {
    using Uint256MsgCodec for bytes;
    using OptionsBuilder for bytes;

    // Event emitted whenever the ball value changes, along with relevant actor
    event BallServed(address indexed sender, uint32 indexed dstEid, uint256 oldBallValue, uint256 newBallValue);
    event BallReceived(address indexed executor, uint256 oldBallValue, uint256 newBallValue);
    event ReturnMessageSent(uint32 indexed dstEid, uint256 ballValue);

    constructor(address _endpoint, address _delegate) OApp(_endpoint, _delegate) Ownable(_delegate) {}

    uint256 public ball = 100000000000000000000;

    /**
     * @dev Internal helper to combine options with memory bytes
     */
    function _combineOptionsMemory(
        uint32 _dstEid,
        uint16 _msgType,
        bytes memory _options
    ) internal view returns (bytes memory) {
        // Create a temporary array to hold options for calldata conversion
        bytes memory tempOptions = _options;
        return this._combineOptionsCalldata(_dstEid, _msgType, tempOptions);
    }

    /**
     * @dev External function to allow calling combineOptions with memory bytes
     */
    function _combineOptionsCalldata(
        uint32 _dstEid,
        uint16 _msgType,
        bytes calldata _options
    ) external view returns (bytes memory) {
        return combineOptions(_dstEid, _msgType, _options);
    }

    /**
     * @notice Sends a message with ABA pattern (ping-pong) to the destination chain.
     * @param _dstEid The endpoint ID of the destination chain.
     * @param _returnOptions Options for the return message (B→A).
     * @param _returnGasEstimate Estimated gas cost for return message execution (B→A).
     * @param _options Additional options for the initial send (A→B).
     * @dev Encodes the message with ABA type and return options, includes return gas in executor options.
     * @return receipt A `MessagingReceipt` struct containing details of the message sent.
     */
    function send(
        uint32 _dstEid,
        bytes calldata _returnOptions,
        uint128 _returnGasEstimate,
        bytes calldata _options
    ) external payable returns (MessagingReceipt memory receipt) {
        // Solidity >0.8 takes care of overflow/underflow
        uint256 oldBall = ball;
        ball = ball - 1;
        emit BallServed(msg.sender, _dstEid, oldBall, ball);

        // Encode ABA message with return options
        bytes memory _message = Uint256MsgCodec.encodeABA(ball, _returnOptions);

        // Build options with return gas included in executor receive options
        // Base gas for receive (~80k) + return gas estimate
        bytes memory abaOptions = OptionsBuilder.newOptions()
            .addExecutorLzReceiveOption(
                180000 + _returnGasEstimate,  // Base gas + return gas estimate
                0
            );

        // Combine with caller options
        bytes memory packedOptions = abi.encodePacked(abaOptions, _options);
        bytes memory combinedOptions = _combineOptionsMemory(
            _dstEid,
            Uint256MsgCodec.ABA_TYPE,
            packedOptions
        );

        receipt = _lzSend(
            _dstEid,
            _message,
            combinedOptions,
            MessagingFee(msg.value, 0),
            payable(msg.sender)
        );
    }

    /**
     * @notice Quotes the gas needed to pay for the ABA omnichain transaction in native gas or ZRO token.
     * @param _dstEid Destination chain's endpoint ID.
     * @param _returnOptions Options for the return message (B→A).
     * @param _returnGasEstimate Estimated gas cost for return message execution (B→A).
     * @param _options Message execution options (e.g., for sending gas to destination).
     * @param _payInLzToken Whether to return fee in ZRO token.
     * @return fee A `MessagingFee` struct containing the calculated gas fee in either the native token or ZRO token.
     * @dev Note: This quotes A→B only. User must separately estimate return gas cost (B→A).
     */
    function quote(
        uint32 _dstEid,
        bytes calldata _returnOptions,
        uint128 _returnGasEstimate,
        bytes calldata _options,
        bool _payInLzToken
    ) public view returns (MessagingFee memory fee) {
        uint256 tempBall = ball - 1;
        
        // Quote initial send (A→B) with ABA message
        bytes memory abaMessage = Uint256MsgCodec.encodeABA(tempBall, _returnOptions);
        
        // Build options with return gas included
        bytes memory abaOptions = OptionsBuilder.newOptions()
            .addExecutorLzReceiveOption(
                180000 + _returnGasEstimate,
                0
            );
        
        bytes memory packedOptions = abi.encodePacked(abaOptions, _options);
        bytes memory combinedOptions = _combineOptionsMemory(
            _dstEid,
            Uint256MsgCodec.ABA_TYPE,
            packedOptions
        );
        
        fee = _quote(_dstEid, abaMessage, combinedOptions, _payInLzToken);
    }

    /**
     * @dev Internal function override to handle incoming messages from another chain.
     * @param _origin A struct containing information about the message sender.
     * @param payload The encoded message payload being received.
     * @param _executor The address of the Executor responsible for processing the message.
     * 
     * Decodes the received payload and processes it. If it's an ABA message, automatically sends a return message.
     */
    function _lzReceive(
        Origin calldata _origin,
        bytes32 /*_guid*/,
        bytes calldata payload,
        address _executor,
        bytes calldata /*_extraData*/
    ) internal override {
        // Decode message (handles both vanilla and ABA formats)
        (uint256 value, uint16 msgType, bytes memory returnOptions) = Uint256MsgCodec.decodeABA(payload);

        uint256 oldBall = ball;
        ball = value;
        emit BallReceived(_executor, oldBall, ball);

        // If ABA type, send response back
        if (msgType == Uint256MsgCodec.ABA_TYPE) {
            // Decrement ball for return message
            ball = ball - 1;
            
            // Encode return message (vanilla type)
            bytes memory returnMessage = Uint256MsgCodec.encode(ball);
            
            // Send back to origin chain using forwarded gas
            // The msg.value contains the forwarded gas from ExecutorLzReceiveOption
            _lzSend(
                _origin.srcEid,
                returnMessage,
                returnOptions,
                MessagingFee(msg.value, 0),  // Use forwarded value
                payable(address(this))  // Refund to contract
            );
            
            emit ReturnMessageSent(_origin.srcEid, ball);
        }
    }
}
