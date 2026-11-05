# scratch.rs

## const SCRATCH_PREFIX
Apart from the MCP's `grammar-mcp.` and its trees under `grammar/`, so
grammar-ssh's sweep and the MCP's never judge each other's directories, though
both may share a base.

## struct ScratchDirs
The two directories the MCP's `scratch.rs` makes, owned rather than kept in a
static, since the controller removes them at grammar-ssh's end
(`controller.rs`).

### fn create
Each base is swept first, so a start cleans after a grammar-ssh killed outright;
a base the two share is swept once. Without a runtime home the transient
directory falls back to the tmp home with the MCP's warning. A transient
directory that can't be made takes the bulk one with it, so a failed start
leaves neither.

### fn remove
Each directory goes only while its marker holds the nonce it was made with; a
failure of either is reported, the other removed regardless.
