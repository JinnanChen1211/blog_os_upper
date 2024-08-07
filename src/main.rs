
#![no_std] // 不链接Rust标准库
#![no_main] // 禁用所有Rust层级的入口点
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};
use cjn_os::interrupts::pics::PICS;
use x86_64::structures::paging::Translate;
use cjn_os::println;
use cjn_os::vga_buffer;

entry_point!(kernel_main);

// 将会在panic时调用
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    cjn_os::hlt_loop();
}


// #[no_mangle] //不重整函数名
// 定义一个符合C调用规范的公开函数 `_start`。由于使用 `-> !` 表明这个函数永不返回.
// pub extern "C" fn _start() -> ! {
// 内核主程序
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Loading Cjn's OS...\n");
    cjn_os::init();
    vga_buffer::print_something();
    // 用于处理x86_64系统中虚拟地址到物理地址的转换
    // 用于表示x86_64系统中的虚拟内存地址
    use x86_64::VirtAddr;
    // 能将一个虚拟地址转换成对应的物理地址
    use  cjn_os::memory;
    // 使用来自于`boot_info.physical_memory_offset`的值创建了一个新的 `VirtAddr`(虚拟内存地址)实例。这个偏移量被用于在物理和虚拟地址之间进行转换
    let phys_mem_offset: VirtAddr = VirtAddr::new(boot_info.physical_memory_offset.clone());
    // new: initialize a mapper
    let mapper = unsafe { memory::init(phys_mem_offset) };
    let addresses = [
        // the identity-mapped vga buffer page
        // 身份映射（identity-mapped）的 VGA 缓冲区页
        // VGA文本模式通常使用的内存地址。
        0xb8000,
        // some code page
        // 某个代码页
        // 表示某个代码页面
        0x201008,
        // some stack page
        // 某个栈页
        // 表示某个栈页面
        0x0100_0020_1a10,
        // virtual address mapped to physical address 0
        // 映射到物理地址 0 的虚拟地址
        // 数组最后一个元素使用克隆来自系统物理内存偏移量(`physical_memory_offset`)作为它们条目，表示操作系统映射到虚拟内存开始处
        boot_info.physical_memory_offset.clone(),
    ];
    // 通过引用（&address）遍历数组中每一项
    for &address in &addresses {
        // 在循环体内，克隆每一项地址，并利用它创建一个新的虚拟内存地址对象
        let virt = VirtAddr::new(address.clone());
        // 调用前面导入标记为不安全(`unsafe`)的函数 `translate_addr`,因直接操纵内存地址若操作不当可能会导致未定义行为。此函数试图使用之前定义好偏移量将我们得到虚拟地址转化成相应物理地址
        let phys = mapper.translate_addr(virt);
        // 循环体内打印出每对虚拟及其翻译后物理地址，以调试格式显示。随后是闭合循环和主方法/程序部分大括号结束符
        println!("{:?} -> {:?}", virt, phys);
    }
    // 进入无限循环防止 `_start` 函数,返回也确保内核不会意外退出到未定义行为状态中去
    cjn_os::hlt_loop();
}
