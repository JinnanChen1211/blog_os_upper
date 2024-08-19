// 引入 `GlobalAlloc` 和 `Layout` 两个模块。`GlobalAlloc` 用于定义全局内存分配器的接口，`Layout` 描述了内存布局
use alloc::alloc::{GlobalAlloc, Layout};
// 引入 `null_mut` 函数，它返回一个空指针（即 `NULL`）
use core::ptr::null_mut;
// 引入x86_64架构相关的分页模块和类型，包括页映射错误、帧分配器、页表标志等，以及虚拟地址类型 `VirtAddr
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use crate::allocator::linked_list::LinkedListAllocator;

// 引入自定义的 `BumpAllocator` 分配器，用于堆内存管理
pub mod bump;
mod linked_list;
// 定义一个通用的锁结构体 `Locked`, 它包含一个互斥锁 (`spin::Mutex`) 来保护内部数据
pub struct Locked<A> {
    inner:spin::Mutex<A>,
}

impl<A> Locked<A> {
    // 为结构体 `Locked` 实现方法：
    // - 构造函数：创建并返回一个新的互斥锁实例。
    // - 锁定方法：获取互斥锁以访问受保护的数据。
    pub const fn new(inner: A) -> Self {
        Locked {
            inner:spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

// 实现向上对齐函数，将地址按给定对齐大小进行对齐。例如，如果地址是1000，且对齐大小是1024，则返回1024
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align.clone() - 1) & !(align - 1)
}

// 定义一个虚拟内存分配器结构体 `Dummy`, 并实现全局分配器接口：
// - 分配方法总是返回空指针。
// - 释放方法触发panic，因为它不应被调用。
pub struct Dummy;
unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("Dealloc should be never called!")
    }
}

#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

pub const HEAP_START: usize = 0x_0001_0000_0000;
pub const HEAP_SIZE: usize = 60 * 1024 * 1024; // 10 MiB

// 初始化堆：
// 1. **计算页面范围**：从起始地址到结束地址，确定需要多少页。
// 2. 创建虚拟地址对象并计算出对应的页对象范围，以便后续映射物理帧
pub fn init_heap (
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
 ) ->Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };
    // 循环遍历每个页面**：
    // - 为每个页面分配物理帧，并检查是否成功。如果失败则返回错误。
    // - 设置页面表标志，使其可读可写。
    // - 使用mapper将虚拟页面映射到物理帧，并刷新缓存以确保映射生效
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }
    // 初始化全局分配器：设置堆起始位置和大小。这一步必须放在安全块里，因为它操作的是裸指针，不受Rust编译器保护
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

#[allow(dead_code)]
pub fn test_allocator() {
    use alloc::boxed::Box;
    use crate::println;
    use alloc::vec::Vec;
    use alloc::vec;
    use alloc::rc::Rc;

    let heap_value = Box::new(831);
    println!("heap_value is at {:p}", heap_value);

    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i)
    }
    println!("vec at {:p}", vec.as_slice());

    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));
}

// 这个代码片段实现了一个简单的内存分配器，并初始化了一个堆。主要包括以下几个步骤：
// 1. 引入必要的库和模块。
// 2. 定义一个线程安全的锁结构`Locked`来保护实际的分配器。
// 3. 实现对齐函数`align_up`。
// 4. 定义一个虚拟内存分配器`Dummy`。
// 5. 声明全局分配器 `ALLOCATOR` 使用 `BumpAllocator`。
// 6. 初始化堆，在虚拟地址空间中为堆分配页面并将其映射到物理帧。

// 通过这些步骤，你可以在操作系统或嵌入式环境中进行内存管理。
