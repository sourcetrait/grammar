# scratch.rs

## const WORKER_PREFIX
Apart from the frontend's `grammar-mcp.`, so a worker's sweep and an MCP's
never judge each other's directories, though both may share a base.

## struct WorkerScratch
The worker's: the directories live from its start to its end, which is its
session's.

### fn create
Each base is swept first, so a start cleans after a worker killed outright;
a base the two share is swept once. Without a runtime home the transient
directory falls back to the tmp home with the MCP's warning. A transient
directory that can't be made takes the bulk one with it, so a failed start
leaves neither.

### fn remove
Each directory goes only while its marker holds the nonce it was made with; a
failure of either is reported, the other removed regardless.
