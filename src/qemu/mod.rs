use x86::io::outb;

// 定义一个枚举类型 `IoPort`，表示常用的I/O端口地址。这里仅包含一个成员 `Com1`, 对应串口1（COM1）的基地址0x3F8。使用 `#[repr(u16)]` 指定该枚举底层存储为 u16 类型
#[repr(u16)]
enum IoPort {
    Com1 = 0x3F8
}

// 定义一个公共函数 `qemu_print`, 用于将字符串内容打印到 QEMU 控制台上
// 将输入字符串转换为字节数组 (`content.as_bytes()`)，然后迭代每个字节
pub fn qemu_print(content: &str) {
    for ch in content.as_bytes() {
        unsafe { outb(IoPort::Com1 as u16, *ch); };
    }
}
