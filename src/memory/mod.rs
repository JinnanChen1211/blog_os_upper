use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

// 引入`x86_64` crate 中的 `PageTable`, `VirtAddr`, 和 `PhysAddr` 类型。这些用于管理虚拟和物理地址以及页面表.
use x86_64::{
    PhysAddr,
    structures::paging::PageTable,
    VirtAddr,
};

use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PhysFrame, Size4KiB};

pub mod graphic_support;

// 初始化偏移页表
//
// 这个函数是危险的，因为其调用的函数具有危险性。
// 详情请见active_level_4_table
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let l4t = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(l4t, physical_memory_offset)
}

// 返回用于激活Level 4页表的引用。
// 必须指出，这个函数是危险的。如果Physical Memory Offset错误给出，将会造成panic。
// 此外，重复调用这个函数也是危险的，因为它会返回静态可变引用。
// 定义一个不安全函数，返回当前活动的Level 4页表（最高级别的页表）的静态可变引用。该函数接受一个表示物理内存偏移量的虚拟地址参数
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    // 引入`Cr3`寄存器模块，它包含指向当前活动页表基地址的指针
    use x86_64::registers::control::Cr3;

    // 读取CR3寄存器获取当前活动Level 4页帧信息，并忽略其中的标志位
    let (level_4_table_frame, _) = Cr3::read();
    // 计算Level 4页表帧在内核映射中虚拟地址，并将其转换为可变原始指针 (`*mut PageTable`类型)。
    let physics = level_4_table_frame.start_address();
    let virt = physical_memory_offset + physics.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    // 通过解引用 (`*`) 并再次取引用 (`&mut`) 的方式将原始指针转换成具有生命周期 `'static'` 的可变引用，并返回。这是危险操作因为若地址计算错误则可能导致未定义行为
    &mut *page_table_ptr // unsafe
}

// 声明一个不安全公共函数，其目标是将给定的虚拟地址转换成对应映射物理地址；如果没有找到映射，则返回None。同样需要传入物理内存偏移量参数
// 将给定的虚拟地址转换为映射的物理地址，或者None（如果不存在的话）,这个函数是危险的。
// 调用者必须保证完整的物理地址已经被映射到虚拟地址上，且在Physical Memory Offset所申明的位置上。
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

// 定义一个私有辅助函数，逻辑上与公共函数相同，但它不被标记为unsafe（虽然实际上仍然是危险操作）
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::structures::paging::page_table::FrameError;
    use x86_64::registers::control::Cr3;

    // 从CR3读当前活跃的L4页帧
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];
    let mut frame = level_4_table_frame;

    // 遍历多级页表
    // 遍历页面多级页表结构来找到给定虚拟地址映射到哪个物理帧。如果此过程中出现问题（如页不存在或不支持巨大页面），将返回None或产生panic异常
    for &index in &table_indexes {
        // 将帧转换为页表引用
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        // 从页表读取位址并更新帧
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("Not Supported HugeFrame")
        };
    }

    // 计算物理地址
    Some(frame.start_address() + u64::from(addr.page_offset()))
}

// 总结逻辑：
// - `active_level_4_table`: 返回当前激活的顶层页表(Level 4)，要求调用者提供正确计算好的物理内存偏移。
// - `translate_addr`: 提供了一种方法来将任意虚拟地址翻译成相应的物理地址，通过遍历从Level 4开始到1级页面每一级对应索引得到最终映射。
// - 这两个功能都很危险：必须确保提供正确且有效的参数和假设条件，否则可能会损坏系统状态或造成崩溃

// 创建一个映射，将给定的页映射到0xb8000
//  **解释**: 定义了一个公共函数 `create_example_mapping`。这个函数接受三个参数：
//   - `page`: 一个页面，表示需要进行映射的虚拟内存页面。
//   - `mapper`: 一个可变引用，指向类型为 `OffsetPageTable` 的内存映射器，用于实际执行页面到物理地址的映射过程。
//   - `frame_allocator`: 帧分配器，以可变引用方式传递，实现了 `FrameAllocator<Size4KiB>` 接口，用于管理和分配内存帧
// FIXME 删了这个函数
// 首先在一个不安全块中调用了内存映射函数，将特定虚拟页 (`page`) 映射到指定物理地址 (`frame`) 上，并设置相应的权限标志 (`flags`)。
// - 然后，它通过 `.expect("Map to Failed")` 来检测这个操作是否成功，如果失败则触发panic并输出错误信息。
// - 最后，通过 `.flush()` 刷新 TLB，确保新的页面映射立即生效
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    // 引入外部包中分页模块所包含的页面表标志，并简化为别名 `Flags` 使用。目的是后续设置分页标志位来控制页表属性，如：是否存在、是否可写等
    use x86_64::structures::paging::PageTableFlags as Flags;
    // 创建一个物理帧并将其指定给变量 `frame`。此物理帧包涵以物理地址 `0xb8000`.  
    // - 使用 `.containing_address()` 方法获取包含该地址范围完整框架
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    //  定义表示该页标志位，通过按位或运算集合状态：
    //  1. **PRESENT** 表示页面存在监控设值会硬件在任何访问发生错误时明确,   【mechanic/MMU usage].
    //  2. ## WRITABLE意味着允许对整个框架读写（必要完成其初特定初始化)
    let flags = Flags::PRESENT | Flags::WRITABLE;

    // 调用者必须保证所请求的帧未被使用
    // `unsafe` 块**: 使用 `unsafe` 块是因为 `mapper.map_to` 方法涉及到底层内存操作，这些操作需要开发者确保其安全性。
    //    - **调用 `map_to` 方法**:
    //    - **`mapper.map_to(...)`**: 调用 `OffsetPageTable` 的 `map_to` 方法，将一个虚拟页（page）映射到一个物理帧（frame），并且应用指定的页面表标志（flags）。
    //    - **参数解释**:
    //      - `page`: 要映射的虚拟页。
    //      - `frame`: 要映射到的物理帧，位于地址 `0xb8000`。
    //      - `flags`: 页面表标志，表示该页在内存中并且可写入。
    //      - `frame_allocator`: 用于分配新的物理帧
    let map_to_result = unsafe {
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    // - **检查结果**:
    //  - **`.expect("Map to Failed")`**: 检查调用是否成功。如果失败，则会触发恐慌 (`panic`) 并输出错误信息 "Map to Failed"。这意味着如果映射操作失败，程序将终止，并报告错误消息。
   
    //  - **刷新 TLB (Translation Lookaside Buffer)**:
    //    - **`.flush()`**: 刷新翻译后备缓冲区 (TLB)，确保新的页面映射立即生效。这一步是必要的，因为 CPU 会缓存最近使用的页面表条目，如果不刷新 TLB，新映射可能不会立刻被使用
    map_to_result.expect("Map_to Failed").flush();
}

// 下面代码片段展示了两种不同类型的帧分配器：
// 1. **EmptyFrameAllocator** 是一个虚拟、空实现，它用于示例或测试目的，不实际进行任何内存分配操作。
// 2. **BootInfoFrameAllocator** 是基于引导加载程序提供的信息来管理和返回可用物理帧的实际实现。它使用了包含系统启动时检测到的所有可用和不可用内存区域信息的数据结构，以便进行有效合理地管理动态资源

// 虚拟的帧分配器
// 这是一个虚拟的（空的）帧分配器，用于示例或测试目的，不实际分配任何内存
// 定义一个公开的空结构体 `EmptyFrameAllocator`，它不包含任何字段。这只是一个占位符，用于实现特定接口
pub struct EmptyFrameAllocator;

// 为 `EmptyFrameAllocator` 实现 `FrameAllocator<Size4KiB>` 接口。这个接口用于管理物理内存帧。
// - **注意**：因为这个实现涉及底层内存操作，所以标记为 `unsafe`
unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    // 在 `allocate_frame` 方法中，总是返回 `None`，表示这个分配器从不实际分配任何内存帧。这符合 "虚拟" 分配器的预期行为，仅用于测试或占位
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        None
    }
}

// 帧分配器，返回BootLoader的内存映射中的可用帧
// 这是一个根据引导加载程序提供的内存映射来返回可用物理帧的实际帧分配器
// - 定义一个公开结构体 `BootInfoFrameAllocator`。
//   - 包含两个字段：
//   - `memory_map`: 引用到静态生命周期（程序运行期间一直存在）的 `MemoryMap`。该字段保存了引导加载程序传递过来的内存布局信息。
//   - `next`: 一个无符号整数，用于追踪下一个可用物理帧的位置索引。例如，可以通过此索引遍历和分配物理内存框架
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    // 使用传递的内存映射创建一个帧分配器
    // 函数不安全，因为调用者必须保证memory_map的正确性
    // - **功能**：这个函数使用传递给它的内存映射（`memory_map`）来初始化一个 `BootInfoFrameAllocator` 实例。
    // - **不安全原因**：标记为 `unsafe`，因为调用者必须确保传递的 `memory_map` 是有效且正确的，否则会导致未定义行为。
    // - **字段初始化**：
    // - `memory_map`: 存储传入的内存映射引用。
    // - `next`: 初始化为0，用于跟踪下一个可用帧的位置
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    // 返回一个可用帧的迭代器
    // 定义一个方法 `usable_frames`，它返回一个迭代器，该迭代器生成所有可用的物理内存帧
    fn usable_frames(&self) -> impl Iterator<Item=PhysFrame> {
        // 获取内存中的可用区域
        // 从 `memory_map` 中获取所有内存区域，并生成一个迭代器 `regions`
        let regions = self.memory_map.iter();
        // 使用过滤器筛选出所有标记为 "Usable" 的内存区域，即那些可以用于分配物理帧的区域
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        // 将这些区域映射到他们的地址范围内
        // 将每个可用区域映射成其对应的地址范围，从起始地址到结束地址。这一步生成了多个地址范围（区间）
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        // 转换为帧起始位置的迭代器
        // 使用 `flat_map` 方法，将每个地址区间按 4096 字节（即4KiB）的步长进行遍历，生成包含所有物理帧起始地址的迭代器。
        // - `step_by(4096)` 确保每次步进大小为一页（4KiB），因为每个物理帧通常是4KiB大小
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // 通过帧起始位置创建PhysFrame类实例
        // 将上述步骤中得到的每个物理帧起始地址转换成 `PhysFrame` 实例。最终返回一个包含所有可用物理框架的迭代器
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

// 这段代码展示了如何基于引导加载程序提供的信息来管理和分配系统启动时检测到的一系列可用物理内存框架:
// 1. 定义并初始化空虚拟分配器和实际有效性依据boot数据之映射源；
// 2. 利用了 Rust 强大泛型、闭包与标准库组件，构建符合逻辑完备带有安全措施之资源管理模块；
// 3. 为后继调用者提供必要接口确保在无缝切换低级别平台硬件资源时仍能保证稳定运行
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    // 为 `BootInfoFrameAllocator` 实现 `FrameAllocator<Size4KiB>` 接口。这个接口定义了分配物理帧的方法。
    //- **注意**: 因为涉及底层内存操作，所以整个实现被标记为不安全 (`unsafe`)
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        // 使用上面定义的 `usable_frames` 方法获取下一个可用物理框架，通过索引值 `self.next` 来选择具体哪一帧，并返回该帧。如果没有更多可用框架，则返回 None
        let frame = self.usable_frames().nth(self.next.clone());
        self.next += 1;
        frame
    }
}
