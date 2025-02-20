# Networking Protocol Documentation

This document outlines the key protocols used in the networking layer.

## Handshake Protocol

The handshake protocol ensures mutual authentication between peers before allowing protocol messages.

```mermaid
sequenceDiagram
    participant A as Peer A
    participant B as Peer B

    Note over A,B: Initial TCP/QUIC Connection Established

    Note over A: Create handshake message with:<br/>1. A's peer ID<br/>2. Current timestamp
    Note over A: Sign(A_id | B_id | timestamp)
    A->>+B: HandshakeRequest {
        public_key: A_pub,
        signature: sign(msg),
        msg: HandshakeMessage {
            sender: A_id,
            timestamp: now
        }
    }

    Note over B: 1. Verify timestamp is fresh<br/>2. Verify A_pub derives to A_id<br/>3. Verify signature<br/>4. Store A's public key

    Note over B: Create handshake message with:<br/>1. B's peer ID<br/>2. Current timestamp
    Note over B: Sign(B_id | A_id | timestamp)
    B-->>-A: HandshakeResponse {
        public_key: B_pub,
        signature: sign(msg),
        msg: HandshakeMessage {
            sender: B_id,
            timestamp: now
        }
    }

    Note over A: 1. Verify timestamp is fresh<br/>2. Verify B_pub derives to B_id<br/>3. Verify signature<br/>4. Store B's public key

    Note over A,B: ✓ Handshake Complete
    Note over A,B: ✓ Protocol Messages Allowed
```

### Handshake States

```mermaid
stateDiagram-v2
    direction LR
    [*] --> Connected: New Connection

    Connected --> OutboundPending: Send Handshake
    Connected --> InboundPending: Receive Handshake

    OutboundPending --> Verifying: Valid Response
    OutboundPending --> Failed: Invalid/Timeout

    InboundPending --> Verifying: Valid Request
    InboundPending --> Failed: Invalid/Timeout

    Verifying --> Verified: All Checks Pass
    Verifying --> Failed: Checks Fail

    Verified --> [*]: Connection Closed
    Failed --> [*]: Connection Closed

    note right of Connected
        Initial TCP/QUIC connection established
    end note

    note right of Verifying
        Checks:
        1. Timestamp fresh
        2. PubKey matches PeerId
        3. Signature valid
        4. Key whitelisted
    end note

    note right of Verified
        Both peers authenticated
        Protocol messages allowed
    end note
```

## Protocol Message Exchange

After handshake completion, peers can exchange protocol messages through direct P2P or broadcast channels.

```mermaid
sequenceDiagram
    participant A as Peer A (Verified)
    participant B as Peer B (Verified)

    Note over A,B: ✓ Handshake Completed

    A->>+B: InstanceMessageRequest {
        protocol: String,
        payload: Vec<u8>,
        metadata: Option<Vec<u8>>
    }

    alt Success Case
        B-->>-A: InstanceMessageResponse::Success {
            data: Option<Vec<u8>>
        }
    else Protocol Response
        B-->>-A: InstanceMessageResponse::Protocol {
            data: Vec<u8>
        }
    else Error Case
        B-->>-A: InstanceMessageResponse::Error {
            code: u16,
            message: String
        }
    end
```

### Message Flow States

```mermaid
stateDiagram-v2
    direction LR
    [*] --> Handshaked: Peers Verified

    Handshaked --> RequestPending: Send Request
    RequestPending --> Processing: Request Received

    Processing --> ResponseSent: Success/Protocol
    Processing --> ErrorSent: Error

    ResponseSent --> Handshaked: Complete
    ErrorSent --> Handshaked: Complete

    Handshaked --> [*]: Connection Closed

    note right of Handshaked
        Peers authenticated
        Ready for messages
    end note

    note right of Processing
        Validating request
        Processing payload
    end note
```

## Protocol Details

### Handshake Protocol

- Initiated on first connection
- Mutual authentication using public key cryptography
- Signatures verify peer identity and ownership
- Timestamps prevent replay attacks
- Timeouts after 30 seconds
- Handles concurrent handshakes gracefully

### Protocol Message Types

- Direct P2P messages:
  - Targeted to specific peer
  - Requires peer verification
  - Guaranteed delivery attempt
- Broadcast messages:
  - Sent to all peers
  - Uses gossipsub protocol
  - Best-effort delivery

### Security Features

- Peer verification before message acceptance
- Public key to peer ID verification
- Timestamp-based replay protection
- Signature verification for handshakes
- Banned peer tracking
- Connection limits
- Protocol version validation
