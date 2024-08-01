
#![no_std] // 不链接Rust标准库
#![no_main] // 禁用所有Rust层级的入口点

use core::panic::PanicInfo;

// 将会在panic时调用
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop{}
}

static HELLO: &[u8] = b"Start os now ...";

#[no_mangle] //不重整函数名
// 定义一个符合C调用规范的公开函数 `_start`。由于使用 `-> !` 表明这个函数永不返回.
pub extern "C" fn _start() -> ! {
    // 定义一个可变指针，指向VGA文本模式缓冲区的开始地址（在 x86 PC上通常都是 0xb8000.
    let vga_buffer = 0xb8000 as *mut u8;
    for (i ,&byte) in HELLO.iter().enumerate() {
        // 因为进行了裸指针操作，所以必须包含在 `unsafe` 块中
        unsafe {
            // 将字符值写进 VGA 缓冲区对应位置
            *vga_buffer.offset(i as isize * 2) = byte;
            // 每个字符后跟着属性字节，在这里设置颜色代码为蓝色 (背景黑色)
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }
    // 进入无限循环防止 `_start` 函数,返回也确保内核不会意外退出到未定义行为状态中去
    loop{}
}
