const HEAP_START: usize = 0x_4444_4444_0000;
const HEAP_SIZE: usize = 100 * 1024; // 100 KiB
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, Size4KiB, mapper::MapToError};
use crate::VirtAddr;
use bitflags::bitflags;
use multiboot2::{ElfSection, ElfSectionFlags};

bitflags! {
    pub struct EntryFlags: u64 {
        const PRESENT = 1 << 0;
        const WRITABLE = 1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLED = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const HUGE_PAGE = 1 << 7;
        const GLOBAL = 1 << 8;
        const NO_EXECUTE = 1 << 63;
    }
}

impl EntryFlags {
    pub fn from_elf_section_flags(section: &ElfSectionFlags) -> EntryFlags {
        let mut flags: EntryFlags = EntryFlags::empty();
        if section.contains(ElfSectionFlags::ALLOCATED) {
            flags = flags | EntryFlags::PRESENT;
        }
        if section.contains(ElfSectionFlags::WRITABLE) {
            flags = flags | EntryFlags::WRITABLE;
        }
        if section.contains(ElfSectionFlags::EXECUTABLE) {
            flags = flags | EntryFlags::NO_EXECUTE;
        }
        flags
    }
}


pub fn init_heap(frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + (HEAP_SIZE as usize).try_into().unwrap() - 1;
        let heap_start_page: Page<Size4KiB> = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = EntryFlags::PRESENT | EntryFlags::WRITABLE;
        unsafe {
            //Mapper::new().map_to(&page, frame, flags, frame_allocator)
            // map frames
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }


    Ok(())
}

use linked_list_allocator::LockedHeap;


#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
