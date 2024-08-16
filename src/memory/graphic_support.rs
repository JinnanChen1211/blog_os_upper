// 引入 `x86_64` 库中的物理地址 (`PhysAddr`) 和虚拟地址 (`VirtAddr`) 类型，以及分页相关的模块和类型，包括帧分配器、映射器、偏移页表、页面、物理帧和4KiB大小的页面
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PhysFrame, Size4KiB};

// 配置区域
// 定义一个常量 `NEEDED_PAGE_NUM`，表示需要映射的页面数量为352
const NEEDED_PAGE_NUM: usize = 352;
// 定义一个常量 `START_ADDR`，表示显存的起始物理地址为0x000A_0000（通常为VGA兼容显存区域）
const START_ADDR: u64 = 0x000A_0000;
// 定义一个公共常量 `START_VIRT_ADDR`，表示显存映射到虚拟内存空间的起始地址为0xC000_0000
pub const START_VIRT_ADDR: u64 = 0xC000_0000;

// 初始化显存
// 定义一个函数 `create_graphic_memory_mapping` 用于初始化显卡显存映射。参数包括：
// - 一个可变引用 `mapper` 指向偏移页表。
// - 一个可变引用 `frame_allocator` 实现了帧分配器接口。
// - 显卡显存起始物理地址 `start_physic_addr`.
pub fn create_graphic_memory_mapping(
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    start_physic_addr: u64
) {
    // 引入并别名化分页标志（Flags），用于设置页面属性
    use x86_64::structures::paging::PageTableFlags as Flags;
    // 循环映射每个页面
    // 对于每个需要映射的页面：
    // - 创建包含指定虚拟地址的页面对象。
    // - 创建包含指定物理地址的物理帧对象。
    // - 设置页面标志，使其可用且可写
    for i in 0..NEEDED_PAGE_NUM {
        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(START_VIRT_ADDR + 0x1000 * i as u64));
        let frame = PhysFrame::containing_address(PhysAddr::new(start_physic_addr + 0x1000 * i as u64));
        let flags = Flags::PRESENT | Flags::WRITABLE;
        // 执行不安全操作将虚拟页映射到物理帧：
        // - 使用提供的页表管理器和帧分配器进行实际内存映射操作。
        // - 如果映射失败，则抛出错误信息 "Map_to_GraphicMemory Failed" 并终止程序。
        // - 成功后刷新TLB缓存，以确保新映射生效
        let map_to_result = unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)
        };
        map_to_result.expect("Map_to_GraphicMemory Failed").flush();
    }
}

// 总结：

// 本代码片段实现了对显卡显存区域进行初始化和内存映射，其主要功能包括：
// 1. **基本设置**：导入必要库和模块，定义所需常量以确定要使用的内存区域及其大小。
// 2. **核心功能**:
//    - 定义函数，用于将一段连续物理内存区域（即显卡显存在内核空间中）通过分页机制逐一映射到虚拟地址空间中.
//    - 使用循环依次处理每个需要被映射到虚拟空间中的页，通过计算得到对应于这些页的位置.
//    - 设置必要属性使这些页“存在”且“可写”.
// 3. **安全性与调试**：使用不安全块执行实际硬件交互操作，并在失败时提供明确错误提示
