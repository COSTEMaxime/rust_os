use super::{align_up, Locked};
use alloc::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};


struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode {size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    head: ListNode
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0)
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    // add a new free memory region to the list
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // ensure that the memory region is capable of holding ListNode
        assert!(align_up(addr, mem::align_of::<ListNode>()) == addr);
        assert!(size >= mem::size_of::<ListNode>());

        // create a new node
        let mut node = ListNode::new(size);
        // node.next = self.head.next
        // self.head.next = None
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        // write `node` in the `addr` memory region (because `node` is currently on the stack)
        node_ptr.write(node);
        // link `node` with `head`
        self.head.next = Some(&mut *node_ptr);
    }

    fn find_region(&mut self, size: usize, align: usize)
        -> Option<(&'static mut ListNode, usize)>
    {
        // ref to the current node
        let mut current = &mut self.head;

        // get next region
        while let Some(ref mut region) = current.next {
            // if region is large enough for the alloc
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // go to next region
                current = current.next.as_mut().unwrap();
            }
        }

        // no suitable region found
        None
    }

    fn alloc_from_region(region: &ListNode, size: usize, align: usize)
        -> Result<usize, ()>
    {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // region too small
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // rest of the region too small to hold a ListNode
            // because if the allocation does not fit the region, a new "free" region will be created
            // this new free region will need to store its own ListNode
            return Err(());
        }

        Ok(alloc_start)
    }

    // adjust the given layout so that the resulting allocated memory region is also capable os storing a `ListNode`
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            // increase the alignment to the alignment of `ListNode`
            .align_to(mem::align_of::<ListNode>())
            .expect("adjustment aligment failed")
            // round up the size to a multiple of the alignment
            // (so that the next memory block will have the correct alignmentto store a `ListNode`)
            .pad_to_align();
        
        // max() to force the allocated size to be min size_of<ListNode>()
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        }
        else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(layout);
        // add deallocated region to the free list
        self.lock().add_free_region(ptr as usize, size)
    }
}