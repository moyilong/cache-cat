# CESP Protocol

## Why Design a New Protocol?

The RESP protocol cannot satisfy several requirements.

1. RESP does not address head-of-line blocking because Redis effectively eliminates this issue through its execution model. Redis commands are executed serially, and the execution cost of each command is relatively similar.

   However, if Cache-cat continues to use the same pipelined design, this can become a problem. In Raft, read requests can use mechanisms such as lease reads and therefore do not necessarily need to go through a complete consensus round. In most cases, it is undesirable for read and write requests to block each other.

   This problem can be solved by assigning a request ID to every request. Both the client and the server need to be adapted accordingly.

2. By default, Redis does not allow clients to explicitly specify the node from which a read should be performed or the read consistency mode to use. For example, clients cannot explicitly require a lease read or a ReadIndex-based read.

   In theory, this could be addressed by adding new commands to RESP.

3. RESP cannot dynamically expose the state of the cluster. High availability can only be implemented through Redis-compatible Sentinel-related commands.

   However, Sentinel commands and the failover mechanisms currently implemented by Redis clients do not align well with the semantics of the Raft algorithm. Redis Sentinel itself is decentralized.

For these reasons, Cache-cat has decided to develop its own protocol to address the issues above. The protocol is named **CESP (CachEcat Serialization Protocol)**.

CESP does not modify any existing Redis operation semantics, nor does it introduce additional RESP-compatible commands that do not already exist in Redis. All new functionality is implemented through the new protocol.

The protocol will eventually support all major programming languages.

CESP is not backward-compatible with older versions of RESP and uses a completely separate port for handling requests.

## Protocol Encoding

Most of the protocol semantics remain consistent with the mature RESP3 protocol.

CESP extends the original RESP message format by adding a fixed `@` prefix, followed by a 32-bit, or 4-byte, request ID.

If the request is a read request, **lease read** is used by default.

If the message starts with `^`, the first byte that follows specifies the read consistency level:

* `0`: Lease read
* `1`: ReadIndex
* `2`: Allow any form of read, including reads from follower nodes

The following 4 bytes still represent the request ID.

The request format is therefore:

`[@][read mode][request ID][remaining part of the original RESP protocol]`

For responses, `@` is always used as the prefix, followed by the 32-bit request ID.
