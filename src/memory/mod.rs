// 引入`x86_64` crate 中的 `PageTable`, `VirtAddr`, 和 `PhysAddr` 类型。这些用于管理虚拟和物理地址以及页面表.
use x86_64::{
    structures::paging::PageTable,
    VirtAddr,
    PhysAddr
};

use x86_64::structures::paging::OffsetPageTable;
/// 初始化偏移页表
///
/// 这个函数是危险的，因为其调用的函数具有危险性。
/// 详情请见active_level_4_table
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let l4t = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(l4t,physical_memory_offset)
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
        let table = unsafe {&*table_ptr};

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
