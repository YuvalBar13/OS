const HEAP_START: usize = 0x_4444_4444_0000;
const HEAP_SIZE: usize = 100 * 1024; // 100 KiB
use x86_64::structures::paging::{mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, Page, Size4KiB};
use crate::VirtAddr;
use x86_64::structures::paging::PageTableFlags as Flags;

pub fn init_heap(frame_allocator: &mut impl FrameAllocator<Size4KiB>, mapper: &mut OffsetPageTable
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
        let flags = Flags::PRESENT | Flags::WRITABLE;
        unsafe {
            let map_to_result = unsafe {
                // FIXME: this is not safe, we do it only for testing
                mapper.map_to(page, frame, flags, frame_allocator)
            };
            map_to_result.expect("map_to failed").flush();
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
