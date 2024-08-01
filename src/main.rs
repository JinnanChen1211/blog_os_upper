
#![no_std] // 不链接Rust标准库
#![no_main] // 禁用所有Rust层级的入口点


mod vga_buffer;
use core::panic::PanicInfo;
#[warn(unused_imports)]
use crate::vga_buffer::Writer;

// 将会在panic时调用
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop{}
}

static HELLO: &[u8] = b"Start os now ...";

#[no_mangle] //不重整函数名
// 定义一个符合C调用规范的公开函数 `_start`。由于使用 `-> !` 表明这个函数永不返回.
pub extern "C" fn _start() -> ! {
    vga_buffer::print_something();
    // 进入无限循环防止 `_start` 函数,返回也确保内核不会意外退出到未定义行为状态中去
    loop{}
}
