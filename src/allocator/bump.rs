use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;

use super::{Locked, align_up};

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }
    // 根据给定堆区间范围初始化Bump Allocator
    // 很显然，这个方法是不安全的，因为给定的区间需要确保未被使用，此外这个函数也不能被多次调用
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start.clone();
        self.heap_end = heap_start + heap_size;
        self.next = heap_start.clone();
    }
}

// 实现 GlobalAlloc trait，用于全局分配器接口
unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    // 分配内存块的方法，根据给定的布局（大小和对齐方式）
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // 获取锁以确保线程安全
        let mut bump = self.lock();
        // 对下一个可用地址进行对齐处理
        let alloc_start = align_up(bump.next.clone(), layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            // 检查是否有溢出风险，如果没有返回结束地址
            Some(end) => end,
            // 如果有溢出，返回空指针表示分配失败
            None => return ptr::null_mut(),
        };
        // 检查是否超出堆区间范围
        if alloc_end > bump.heap_end.clone() {
            // 超出则返回空指针，表示分配失败
            ptr::null_mut()
        } else {
            // 更新下一个可用地址为当前分配结束地址后的位置
            bump.next = alloc_end as usize;
            // 增加已分配计数器
            bump.allocations += 1;
            // 返回开始地址作为分配结果的指针
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // 获取锁以确保线程安全
        let mut bump = self.lock();
        // 减少已分配计数器
        bump.allocations -= 1;
        // 如果所有内存都被释放了，将next重置为heap起始位置，实现"回收"
        if bump.allocations == 0 {
            bump.next = bump.heap_start.clone();
        }
    }
}

// 总结：
// - `BumpAllocator` 是一种简单的内存分配器，通过线性增长来管理内存。
// - 它有四个字段：`heap_start`, `heap_end`, `next`, 和 `allocations`。
// - `new` 方法创建一个新的实例，并初始化所有字段为零。
// - `init` 方法用于设置堆区间及初始状态。由于它直接操作裸指针，所以是 `unsafe` 的。
// - 实现了 `GlobalAlloc` trait，使其可以作为全局内存分配器使用。
// - 内部通过对齐和检查防止越界或溢出的情况发生，并在释放所有内存时重置状态。
