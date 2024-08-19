use alloc::format;

// 引入 `x86` 库中的 `outw` 函数，用于向 I/O 端口写入数据
use x86::io::outw;
// 引入 x86_64 架构相关的分页模块和类型，包括帧分配器、偏移页表以及页面大小
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable, Size4KiB};
use crate::io::pci::{pci_config_read_u32, pci_find_device};
use crate::memory::graphic_support::create_graphic_memory_mapping;
// 引入自定义模块中的函数 `qemu_print`, 用于打印调试信息到 QEMU 控制台
use crate::io::qemu::qemu_print;

// 定义两个常量，表示VBE接口的I/O端口地址（INDEX和DATA）
const VBE_DISPI_IOPORT_INDEX: u16 = 0x01CE;
const VBE_DISPI_IOPORT_DATA: u16 = 0x01CF;

// 定义一个枚举类型，表示不同的VBE寄存器索引。使用u16表示这些索引值，并且允许未使用代码存在（dead code）
#[allow(dead_code)]
#[repr(u16)]
// 注册索引
enum VbeDispiIndex {
    Id = 0,
    Xres,
    Yres,
    Bpp,
    Enable,
    Bank,
    VirtWidth,
    VirtHeight,
    XOffset,
    YOffset,
}

// 定义另一个枚举类型，表示不同颜色深度（bits per pixel, BPP）。同样允许未使用代码存在
#[allow(dead_code)]
#[repr(u16)]
// 位深度
enum VbeDispiBpp {
    _4 = 4,
    _8 = 8,
    _24 = 24,
    _32 = 32,
    // 省略了很多我不可能用得到的深度
}

// 定义一个不安全函数，用于向指定寄存器写入数据。首先向INDEX端口写索引，再向DATA端口写值
unsafe fn bga_write_register(index: u16, value: u16) {
    outw(VBE_DISPI_IOPORT_INDEX, index);
    outw(VBE_DISPI_IOPORT_DATA, value);
}

// 宽屏模式进入函数
pub unsafe fn bga_enter_wide(
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    // 定义进入宽屏模式的不安全方法：
    // - 首先禁用VBE，通过将Enable寄存器设置为0实现
    bga_write_register(VbeDispiIndex::Enable as u16, 0);

    // - 然后设置显示分辨率和颜色深度。
    // - 使用外部模块提供的常量WIDTH和HEIGHT设置X轴/ Y轴分辨率.
    // - 设置颜色深度为24位.
    // 设置显示模式
    bga_write_register(VbeDispiIndex::Xres as u16, super::WIDTH as u16);
    bga_write_register(VbeDispiIndex::Yres as u16, super::HEIGHT as u16);
    bga_write_register(VbeDispiIndex::Bpp as u16, VbeDispiBpp::_32 as u16);

    // 再次启用 VBE，将 Enable 寄存器设置为特殊值以开启图形模式
    bga_write_register(VbeDispiIndex::Enable as u16, 0x41);

    // 获取LFB地址
    // - 查找特定PCI设备(假设厂商ID为1111，设备ID为1234)并获取其线性帧缓冲(LFB)地址.
    //  - 打印调试信息以确认设备及其地址
    let device = pci_find_device(0x1111, 0x1234);
    qemu_print(format!("LFB device is {:?}\n", device).as_str());
    let address = pci_config_read_u32(device.0, device.1, device.2, 0x10);
    qemu_print(format!("We get LFB address:{:?}\n", address).as_str());

    // 初始化显存
    //  最后调用自定义方法初始化显存，即将LFB地址映射到虚拟内存空间中
    create_graphic_memory_mapping(mapper, frame_allocator, address as u64);
}

// ## 总结：

// 本代码片段主要完成以下功能：
// 1. **基本设置**：包括导入必要库和模块，定义常量及枚举类型来表示硬件寄存器及相关参数.
// 2. **核心功能**：
//   - 提供低级别操作接口，如通过I / O端口读写硬件寄存器.
//   - 实现进入宽屏显示模式的方法，通过一系列步骤配置并启用图形显示，然后获取并初始化显卡显存映射.
// 3. **调试辅助**：通过QEMU控制台打印重要调试信息，以便开发过程中验证各步骤是否正确执行成功
