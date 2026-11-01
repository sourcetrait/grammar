//! The session's frame ceilings and its payloads' decode budgets.

/// Each hello, both ways, is held to this ceiling.
pub const HELLO_FRAME_MAX: u32 = 64 * 1024;

/// The session's frame ceiling before negotiation.
pub const FRAME_DEFAULT: u32 = 8 * 1024 * 1024;

/// The session's frame ceiling no negotiation passes.
pub const FRAME_HARD: u32 = 64 * 1024 * 1024;

/// A payload's decode bounds, each checked before what it bounds is allocated.
pub type DecodeBudget = bitcode::Budget;

/// A payload's decode budget before negotiation: 100,000 elements, 32 MiB of heap.
pub const DECODE_BUDGET_DEFAULT: DecodeBudget = DecodeBudget {
    elements: 100_000,
    heap: 32 * 1024 * 1024,
    depth: 128,
};

/// The decoded heap no negotiation passes.
pub const DECODE_HEAP_HARD: u64 = 128 * 1024 * 1024;
