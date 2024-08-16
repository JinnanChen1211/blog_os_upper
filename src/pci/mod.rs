use alloc::format;

use x86::io::{inl, outl};

use crate::qemu::qemu_print;

// 定义两个常量，表示PCI配置空间的地址寄存器和数据寄存器的I/O端口地址
const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

// 读取PCI配置空间
// - 定义一个函数 `pci_config_read_u32`，用于读取指定位置的PCI配置空间。
// - 参数包括总线号 (`bus`)、设备号 (`device`)、功能号 (`function`) 和偏移量 (`offset`)。
// - 构造配置空间地址：
//     - 总线号左移16位。
//     - 设备号左移11位。
//     - 功能号左移8位。
//     - 偏移量按字对齐（最低两位清零）并限定范围为4字节对齐。
//     - 设置最高有效位以启用访问模式 (0x80000000)。

// 不安全块中：
// - 写入构造好的地址到PCI配置地址寄存器。
// - 从PCI配置数据寄存器读取并返回对应的数据。
pub fn pci_config_read_u32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let addr: u32 = ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | ((offset as u32) & 0xFC) | 0x8000_0000u32;
    return unsafe {
        outl(PCI_CONFIG_ADDRESS, addr);
        inl(PCI_CONFIG_DATA)
    };
}

// - 定义一个函数 `pci_find_device`，用于查找特定厂商ID和设备ID的PCI设备。
// - 参数包括目标设备ID (`device_id`) 和厂商ID (`vendor_id`)。返回值为找到的总线号、设备号和功能号（如果未找到，则返回 `(0xFF, 0xFF ,0xFF)`）。
// 构建目标值：
// - 将设备ID左移16位并加上厂商ID，以匹配完整标识符。
// 嵌套循环遍历所有可能组合：
// 1. 遍历所有可能总线（范围从 `0` 到 `255`）。
// 2. 遍历每个总线上最多可达 `31` 个设备位置。
// 3. 遍历每个设备上的最多八种功能（一些多功能卡支持多个功能）。
// 注释掉了调试输出语句：
// 在最内层循环中，
// - 调用前述读取函数检查是否匹配，如果匹配则立即返回其位置（三元组形式：总线、设备、功能）。
// 如果没有找到匹配项，则返回无效值 `(255 ,255 ,255)` 表示失败.
pub fn pci_find_device(device_id: u16, vendor_id: u16) -> (u8, u8, u8) {
    let target = ((device_id as u32) << 16) + vendor_id as u32;
    for bus in 0..=255 {
        for device in 0..32 {
            for function in 0..8 {
                // qemu_print(format!("{},{},{}", bus, device, function).as_str());
                if pci_config_read_u32(bus, device, function, 0) == target {
                    return (bus, device, function);
                }
            }
        }
    }

    // 找不到，找不到
    (0xFF, 0xFF, 0xFF)
}

// ## 总结:

// 本代码片段实现了基本操作来与系统中的 PCI 配置空间进行交互，其主要功能包括:
// 1. **基本设置**：导入必要库和模块以及定义所需常量以确定相关硬件接口.
// 2. **核心功能**:
//    - 提供低级别读写接口，通过 IO 操作实现对 PCI 配置空间特定位置数据进行读写.
//    - 实现查找给定厂商 ID 和/或 Device ID 的具体 PCI 硬件实例位置的方法，通过遍历所有可能组合定位到具体硬件所在的位置.
// 3. **调试辅助**：通过提供基于条件编译输出机制使得在开发/测试阶段能够方便地验证各步骤执行情况及相应结果.
