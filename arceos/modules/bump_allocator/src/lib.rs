#![no_std]

use core::ptr::NonNull;

use allocator::{AllocError, BaseAllocator, ByteAllocator, PageAllocator};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start: usize,
    end: usize,
    b_pos: usize,
    p_pos: usize,
    b_count: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
            b_count: 0,
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.b_pos = start;
        self.end = start + size;
        self.p_pos = start + size;
    }

    fn add_memory(&mut self, _start: usize, _size: usize) -> allocator::AllocResult {
        Err(AllocError::NoMemory) // unsupported
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: core::alloc::Layout) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();
        let alloc_start = (self.b_pos + align - 1) & !(align - 1);
        let alloc_end = alloc_start.checked_add(size).ok_or(AllocError::NoMemory)?;

        if alloc_end > self.p_pos {
            return Err(AllocError::MemoryOverlap);
        }

        self.b_pos = alloc_end;
        self.b_count += 1;
        Ok(unsafe { NonNull::new_unchecked(alloc_start as *mut u8) })
    }

    fn dealloc(&mut self, _pos: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        if self.b_count == 0 {
            return;
        }
        
        self.b_count -= 1;
        if self.b_count == 0 {
            self.b_pos = self.start;
        }
    }

    fn total_bytes(&self) -> usize {
        self.end - self.start
    }

    fn used_bytes(&self) -> usize {
        self.b_pos - self.start
    }

    fn available_bytes(&self) -> usize {
        self.p_pos - self.b_pos
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> allocator::AllocResult<usize> {
        let align = 1 << align_pow2;
        let size = num_pages * PAGE_SIZE;

        let alloc_end = self.p_pos;
        let mut alloc_start = alloc_end.checked_sub(size).ok_or(AllocError::NoMemory)?;
        alloc_start &= !(align - 1);

        if alloc_start < self.b_pos {
            return Err(AllocError::MemoryOverlap);
        }

        self.p_pos = alloc_start;
        Ok(alloc_start)
    }

    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // pages would not be freed
    }

    fn total_pages(&self) -> usize {
        (self.end - self.start) / PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.end - self.p_pos) / PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        (self.p_pos - self.b_pos) / PAGE_SIZE
    }
}
