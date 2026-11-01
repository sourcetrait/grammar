//! The wire's fixed widths, taken from the platform's.

/// A usize is never wider than a u64, so widening one never alters it.
const _: () = assert!(usize::BITS <= u64::BITS);

/// A platform-width count or offset at the wire's width.
pub const fn wide(n: usize) -> u64 {
    n as u64
}
