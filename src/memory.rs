use x86_64::{
    structures::paging::{
        Page,
        PageTable,
        PhysFrame,
        Mapper,
        Size4KiB,
        FrameAllocator,
        UnusedPhysFrame,
        OffsetPageTable,
    },
    VirtAddr,
    PhysAddr,
};

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};


pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table_frame = active_level_4_table(physical_memory_offset);
    // support huge pages
    OffsetPageTable::new(level_4_table_frame, physical_memory_offset)
}

// Get a mutable reference to the active level 4 page table
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    // unsafe
    &mut *page_table_ptr
}

pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>
) {
    use x86_64::structures::paging::PageTableFlags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));

    let unused_frame = unsafe { UnusedPhysFrame::new(frame) };
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    /* map_to :
        - Allocate an anused frame from the 'frame_allocator'
        - Zero the frame to create a new page table
        - Map the entry of the higher level table to that frame
        - Continue with the next table level
    */
    let map_to_result = mapper.map_to(page, unused_frame, flags, frame_allocator);
    map_to_result.expect("map_to failed").flush();
}

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame> {
        None
    }
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    // convert memory_map into an iterator of unused frames
    fn usable_frames(&self) -> impl Iterator<Item = UnusedPhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        // map each region to its address space (transform MemoryRegion to address ranges using ..)
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());

        // transfor to an iterator of frame start addresses
        // step_by = choose every 4096th address (== start address of each frame as the page size is 4KiB)
        // flat_map to get Iterator<Item = u64> instead of Iterator<Item = Iterator<Item = u64>>
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

        // create 'PhysFrame' type for each start address
        // transform to Iterator<Item = PhysFrame>
        let frames = frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)));

        // transform into Iterator<Item = UnusedPhysFrame>
        frames.map(|f| unsafe { UnusedPhysFrame::new(f) })
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}