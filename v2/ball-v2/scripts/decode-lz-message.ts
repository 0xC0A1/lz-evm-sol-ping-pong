import { ethers } from 'ethers'

const RPC_URL = 'https://eth-sepolia.g.alchemy.com/v2/UgBZILPWmtdXrICFmC7r_kmYMukRa0se'
const TX_HASH = '0x6e85c8eed8373bed30cd0ddc3a5f7af71592a759b84cbb50c1fda3c51839fff0'

async function decodeMessage() {
    const provider = new ethers.providers.JsonRpcProvider(RPC_URL)
    const receipt = await provider.getTransactionReceipt(TX_HASH)
    
    if (!receipt) {
        console.error('Transaction receipt not found')
        return
    }
    
    // Find LayerZero PacketSent event
    // PacketSent(bytes encodedPayload, bytes options, address sendLibrary)
    const packetSentTopic = ethers.utils.keccak256(ethers.utils.toUtf8Bytes('PacketSent(bytes,bytes,address)'))
    
    for (const log of receipt.logs) {
        if (log.topics[0] === packetSentTopic || log.address.toLowerCase() === '0x6edce65403992e310a62460808c4b910d972f10f') {
            console.log('Found LayerZero log')
            console.log('Topics:', log.topics.length)
            console.log('Data length:', log.data.length / 2 - 1, 'bytes')
            
            // Decode the event
            const iface = new ethers.utils.Interface([
                'event PacketSent(bytes encodedPayload, bytes options, address sendLibrary)'
            ])
            
            try {
                const decoded = iface.parseLog(log)
                const encodedPayload = ethers.utils.arrayify(decoded.args.encodedPayload)
                console.log('\n=== LayerZero encodedPayload structure ===')
                console.log('Total length:', encodedPayload.length, 'bytes')
                
                // LayerZero v2 encodedPayload structure:
                // - version (1 byte)
                // - srcEid (4 bytes)
                // - sender (32 bytes)
                // - nonce (8 bytes)
                // - dstEid (4 bytes)
                // - receiver (32 bytes)
                // - guid (32 bytes)
                // - message (variable length, ABI-encoded with offset)
                
                let offset = 0
                const version = encodedPayload[offset]
                offset += 1
                console.log('Version:', version)
                
                const srcEid = ethers.BigNumber.from(encodedPayload.slice(offset, offset + 4))
                offset += 4
                console.log('srcEid:', srcEid.toString())
                
                const sender = ethers.utils.hexlify(encodedPayload.slice(offset, offset + 32))
                offset += 32
                console.log('Sender:', sender)
                
                const nonce = ethers.BigNumber.from(encodedPayload.slice(offset, offset + 8))
                offset += 8
                console.log('Nonce:', nonce.toString())
                
                const dstEid = ethers.BigNumber.from(encodedPayload.slice(offset, offset + 4))
                offset += 4
                console.log('dstEid:', dstEid.toString())
                
                const receiver = ethers.utils.hexlify(encodedPayload.slice(offset, offset + 32))
                offset += 32
                console.log('Receiver:', receiver)
                
                const guid = ethers.utils.hexlify(encodedPayload.slice(offset, offset + 32))
                offset += 32
                console.log('GUID:', guid)
                
                console.log('\n=== Remaining payload (should be ABA message) ===')
                const messageBytes = encodedPayload.slice(offset)
                console.log('Message length:', messageBytes.length, 'bytes')
                console.log('Message hex (first 200 chars):', ethers.utils.hexlify(messageBytes).substring(0, 200))
                
                // Now decode the ABA message
                if (messageBytes.length >= 128) {
                    console.log('\n=== Decoding ABA Message ===')
                    
                    // First 32 bytes: uint256 (ball)
                    const ballBytes = messageBytes.slice(0, 32)
                    const ball = ethers.BigNumber.from(ballBytes)
                    console.log('Ball (uint256):', ball.toString())
                    console.log('Ball (hex):', ethers.utils.hexlify(ballBytes))
                    
                    // Next 32 bytes: uint16 padded (msg_type)
                    const msgTypeBytes = messageBytes.slice(32, 64)
                    const msgType = ethers.BigNumber.from(msgTypeBytes.slice(30, 32))
                    console.log('Message Type (uint16):', msgType.toString())
                    console.log('Expected ABA_TYPE: 2')
                    
                    if (msgType.toString() !== '2') {
                        console.log('⚠️  ERROR: Message type mismatch!')
                    }
                    
                    // Next 32 bytes: offset to return_options
                    const offsetBytes = messageBytes.slice(64, 96)
                    const offsetValue = ethers.BigNumber.from(offsetBytes.slice(24, 32))
                    console.log('Return Options Offset:', offsetValue.toString())
                    
                    if (offsetValue.gte(128) && offsetValue.lte(messageBytes.length)) {
                        const offsetNum = Number(offsetValue.toString())
                        const lenBytes = messageBytes.slice(offsetNum, offsetNum + 32)
                        const len = ethers.BigNumber.from(lenBytes.slice(24, 32))
                        console.log('Return Options Length:', len.toString())
                        
                        if (len.gt(0)) {
                            const lenNum = Number(len.toString())
                            const returnOptionsBytes = messageBytes.slice(offsetNum + 32, offsetNum + 32 + lenNum)
                            console.log('Return Options:', ethers.utils.hexlify(returnOptionsBytes))
                        } else {
                            console.log('Return Options: (empty)')
                        }
                    } else {
                        console.log('⚠️  ERROR: Invalid offset!')
                        console.log('  Offset:', offsetValue.toString())
                        console.log('  Message length:', messageBytes.length)
                    }
                }
            } catch (e: any) {
                console.error('Error decoding:', e.message)
            }
        }
    }
}

decodeMessage().catch(console.error)

