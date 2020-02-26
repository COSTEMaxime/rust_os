use super::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::{ptr, ptr::NonNull, mem};

struct ListNode {
    next: Option<&'static mut ListNode>
}

// 2^n because they are also used as the block alignment which must be 2^n
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap
}

impl FixedSizeBlockAllocator {
    pub const fn new() -> Self {
        FixedSizeBlockAllocator {
            list_heads: [None; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty()
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.fallback_allocator.init(heap_start, heap_size);
    }

    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                // remove first node
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        // link head to second node
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    }
                    None => {
                        // no block exists in the list
                        let block_size = BLOCK_SIZES[index];
                        let block_align = block_size;
                        let layout = Layout::from_size_align(block_size, block_align).unwrap();

                        allocator.fallback_alloc(layout)
                    }
                }
            }
            None => allocator.fallback_alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                // create new block
                let new_node = ListNode {
                    next: allocator.list_heads[index].take()
                };

                // assert that the new node have the correct size / alignment to store a `ListNode`
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);

                let new_node_ptr = ptr as *mut ListNode;
                new_node_ptr.write(new_node);
                // set head of the list (which is None bacause we used take()) to the new node
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            }
            None => {
                // allocation created by `fallback_allocator`
                let ptr = NonNull::new(ptr).unwrap();
                allocator.fallback_allocator.deallocate(ptr, layout);
            }
        }
    }
}

fn list_index(layout: &Layout) -> Option<usize> {
    // max() to layout.align()
    let required_block_size = layout.size().max(layout.align());
    // find index of the first block at least as large as `required_block_size`
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}