//! Per-process garbage collector for JAPL.
//!
//! Each JAPL process has its own heap, collected independently. This eliminates
//! global stop-the-world pauses -- GC for one process never blocks another.
//!
//! The collector is generational:
//! - **Nursery (young generation):** Bump allocation with copying collection.
//!   When full, live objects are copied to old gen (Cheney's algorithm).
//! - **Old generation:** Mark-compact collection. Triggered when old gen grows
//!   past a threshold.
//!
//! Since all JAPL values are immutable, no write barriers are needed.

use std::ptr;

/// Default nursery size: 256 KB.
const DEFAULT_NURSERY_SIZE: usize = 256 * 1024;

/// Default old generation initial capacity: 1 MB.
const DEFAULT_OLD_GEN_SIZE: usize = 1024 * 1024;

/// Default GC threshold ratio: trigger major GC when old gen is 75% full.
const DEFAULT_GC_THRESHOLD_RATIO: f64 = 0.75;

/// Per-process heap with generational garbage collection.
///
/// The heap is owned by a single process and never shared. This allows
/// collection without any cross-process synchronization.
pub struct ProcessHeap {
    /// Young generation: bump-allocated region.
    nursery: Vec<u8>,
    /// Current allocation pointer within the nursery.
    nursery_top: usize,
    /// Old generation: long-lived objects.
    old_gen: Vec<u8>,
    /// Bytes used in the old generation.
    old_gen_used: usize,
    /// Threshold (in bytes) at which to trigger major GC.
    gc_threshold: usize,
    /// Collection statistics.
    stats: GcStats,
}

/// Statistics about garbage collection activity.
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    /// Number of nursery (minor) collections performed.
    pub nursery_collections: u64,
    /// Number of major (old gen) collections performed.
    pub major_collections: u64,
    /// Total bytes allocated over the process lifetime.
    pub total_allocated: u64,
    /// Current live bytes in the heap.
    pub current_live: u64,
}

/// Header prepended to each heap-allocated object.
///
/// This is used by the GC to track object metadata during collection.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ObjectHeader {
    /// Size of the object payload in bytes (excluding header).
    pub size: u32,
    /// GC mark/forwarding flag.
    pub flags: u32,
}

impl ObjectHeader {
    /// Flag indicating the object has been marked as live.
    pub const MARK_BIT: u32 = 0x01;
    /// Flag indicating the object has been forwarded (nursery -> old gen).
    pub const FORWARDED: u32 = 0x02;

    pub fn new(size: u32) -> Self {
        ObjectHeader { size, flags: 0 }
    }

    pub fn is_marked(&self) -> bool {
        self.flags & Self::MARK_BIT != 0
    }

    pub fn set_marked(&mut self) {
        self.flags |= Self::MARK_BIT;
    }

    pub fn clear_marked(&mut self) {
        self.flags &= !Self::MARK_BIT;
    }

    pub fn is_forwarded(&self) -> bool {
        self.flags & Self::FORWARDED != 0
    }

    pub fn set_forwarded(&mut self) {
        self.flags |= Self::FORWARDED;
    }
}

const HEADER_SIZE: usize = std::mem::size_of::<ObjectHeader>();

impl ProcessHeap {
    /// Create a new process heap with default sizes.
    pub fn new() -> Self {
        Self::with_sizes(DEFAULT_NURSERY_SIZE, DEFAULT_OLD_GEN_SIZE)
    }

    /// Create a new process heap with specified nursery and old gen sizes.
    pub fn with_sizes(nursery_size: usize, old_gen_size: usize) -> Self {
        let gc_threshold = (old_gen_size as f64 * DEFAULT_GC_THRESHOLD_RATIO) as usize;
        ProcessHeap {
            nursery: vec![0u8; nursery_size],
            nursery_top: 0,
            old_gen: vec![0u8; old_gen_size],
            old_gen_used: 0,
            gc_threshold,
            stats: GcStats::default(),
        }
    }

    /// Allocate `size` bytes from the nursery.
    ///
    /// Returns a pointer to the start of the allocated region (after the
    /// object header). If the nursery is full, triggers a minor GC first.
    ///
    /// # Safety
    ///
    /// The returned pointer is valid until the next GC cycle. Callers must
    /// ensure they have registered roots before allocating.
    pub fn alloc(&mut self, size: usize) -> *mut u8 {
        let total_size = HEADER_SIZE + align_up(size, 8);

        // Check if nursery has space
        if self.nursery_top + total_size > self.nursery.len() {
            self.minor_gc();

            // If still not enough space after GC, grow the nursery
            if self.nursery_top + total_size > self.nursery.len() {
                let new_size = (self.nursery.len() * 2).max(self.nursery_top + total_size);
                self.nursery.resize(new_size, 0);
            }
        }

        let header_ptr = self.nursery_top;
        let obj_ptr = header_ptr + HEADER_SIZE;

        // Write the object header
        let header = ObjectHeader::new(size as u32);
        unsafe {
            ptr::write(
                self.nursery.as_mut_ptr().add(header_ptr) as *mut ObjectHeader,
                header,
            );
        }

        self.nursery_top = header_ptr + total_size;
        self.stats.total_allocated += total_size as u64;
        self.stats.current_live += total_size as u64;

        unsafe { self.nursery.as_mut_ptr().add(obj_ptr) }
    }

    /// Perform a minor (nursery) garbage collection.
    ///
    /// In a full implementation, this would trace roots and copy live objects
    /// from the nursery to old gen. For now, we simulate the collection by
    /// promoting all nursery data to old gen and resetting the nursery.
    pub fn minor_gc(&mut self) {
        self.stats.nursery_collections += 1;

        // Copy nursery contents to old gen (simplified: copies everything)
        let nursery_used = self.nursery_top;
        if nursery_used > 0 {
            // Ensure old gen has capacity
            if self.old_gen_used + nursery_used > self.old_gen.len() {
                let new_size = (self.old_gen.len() * 2).max(self.old_gen_used + nursery_used);
                self.old_gen.resize(new_size, 0);
                self.gc_threshold =
                    (self.old_gen.len() as f64 * DEFAULT_GC_THRESHOLD_RATIO) as usize;
            }

            self.old_gen[self.old_gen_used..self.old_gen_used + nursery_used]
                .copy_from_slice(&self.nursery[..nursery_used]);
            self.old_gen_used += nursery_used;
        }

        // Reset nursery
        self.nursery_top = 0;

        // Check if we should trigger a major GC
        if self.old_gen_used > self.gc_threshold {
            self.major_gc();
        }
    }

    /// Perform a major (old generation) garbage collection.
    ///
    /// In a full implementation, this would do mark-compact on the old gen.
    /// For now, we just update stats. Since JAPL values are immutable, a
    /// production implementation would walk the root set, mark reachable
    /// objects, and compact the live data.
    pub fn major_gc(&mut self) {
        self.stats.major_collections += 1;
        // In a full implementation: mark from roots, compact live objects.
        // For now, old_gen_used represents an upper bound.
        self.stats.current_live = self.old_gen_used as u64;
    }

    /// Total memory used by this process heap (nursery + old gen).
    pub fn memory_used(&self) -> usize {
        self.nursery_top + self.old_gen_used
    }

    /// Total memory capacity of this process heap.
    pub fn memory_capacity(&self) -> usize {
        self.nursery.len() + self.old_gen.len()
    }

    /// Get GC statistics for this heap.
    pub fn stats(&self) -> &GcStats {
        &self.stats
    }

    /// Reset the heap entirely. Used when a process is restarted by a supervisor.
    pub fn reset(&mut self) {
        self.nursery_top = 0;
        self.old_gen_used = 0;
        self.stats = GcStats::default();
    }
}

impl Default for ProcessHeap {
    fn default() -> Self {
        Self::new()
    }
}

/// Align `n` up to the next multiple of `align`.
fn align_up(n: usize, align: usize) -> usize {
    (n + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_creation() {
        let heap = ProcessHeap::new();
        assert_eq!(heap.nursery_top, 0);
        assert_eq!(heap.old_gen_used, 0);
        assert_eq!(heap.memory_used(), 0);
    }

    #[test]
    fn test_alloc_basic() {
        let mut heap = ProcessHeap::new();
        let ptr = heap.alloc(64);
        assert!(!ptr.is_null());
        assert!(heap.nursery_top > 0);
        assert!(heap.stats.total_allocated > 0);
    }

    #[test]
    fn test_alloc_multiple() {
        let mut heap = ProcessHeap::new();
        let p1 = heap.alloc(32);
        let p2 = heap.alloc(64);
        assert_ne!(p1, p2);
        assert!(heap.nursery_top >= 32 + 64 + 2 * HEADER_SIZE);
    }

    #[test]
    fn test_minor_gc_triggers() {
        // Use a small nursery to force GC
        let mut heap = ProcessHeap::with_sizes(128, 4096);
        // Allocate more than nursery can hold
        for _ in 0..10 {
            heap.alloc(32);
        }
        assert!(heap.stats.nursery_collections > 0);
    }

    #[test]
    fn test_major_gc_triggers() {
        // Small old gen to force major GC
        let mut heap = ProcessHeap::with_sizes(64, 128);
        for _ in 0..20 {
            heap.alloc(32);
        }
        assert!(heap.stats.major_collections > 0);
    }

    #[test]
    fn test_heap_reset() {
        let mut heap = ProcessHeap::new();
        heap.alloc(64);
        heap.alloc(128);
        heap.reset();
        assert_eq!(heap.nursery_top, 0);
        assert_eq!(heap.old_gen_used, 0);
        assert_eq!(heap.stats.nursery_collections, 0);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 8), 0);
        assert_eq!(align_up(1, 8), 8);
        assert_eq!(align_up(7, 8), 8);
        assert_eq!(align_up(8, 8), 8);
        assert_eq!(align_up(9, 8), 16);
    }
}
