
#![no_std] // 不链接Rust标准库
#![no_main] // 禁用所有Rust层级的入口点
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};
use cjn_os::interrupts::pics::PICS;
#[warn(unused_imports)]
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
fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    vga_buffer::print_something();
    // 将变量设置为指向第四级页表(Page Table Level 4)开始位置的指针。根据x86_64架构，第四级页表通常位于此虚拟地址末尾处。
    // 通过循环遍历前十个第四级页表项，并且使用不安全（unsafe）代码块，因为直接操作裸指针存在风险。每轮迭代读取相应内存位置上存储的u64整数值作为条目，并打印其索引和16进制形式内容
    use x86_64::structures::paging::PageTable;
    let level_4_table_ptr = 0xffff_ffff_ffff_f000 as *const PageTable;
    let level_4_table = unsafe {&*level_4_table_ptr};
    for i in 0..10 {
        println!("Entry {}: {:?}", i, level_4_table[i]);
    }
    // 进入无限循环防止 `_start` 函数,返回也确保内核不会意外退出到未定义行为状态中去
    cjn_os::hlt_loop();
}
