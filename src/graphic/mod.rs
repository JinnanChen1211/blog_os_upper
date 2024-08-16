// 引入 `alloc` 库中的 `format` 宏，用于格式化字符串
use alloc::format;
// 引入 `core` 库中的 `min` 函数，用于计算两个值的较小值
use core::cmp::min;
// 引入 `embedded_graphics` 库中的颜色类型 `Rgb888` 和预导出的所有内容（prelude），以及另一个颜色类型 `Bgr888`
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use embedded_graphics::pixelcolor::Bgr888;
// 引入 `lazy_static` 宏，用于声明静态变量并进行延迟初始化
use lazy_static::lazy_static;
// 引入 `spin` 库中的互斥锁 (`Mutex`)，用于实现线程安全的共享数据访问
use spin::Mutex;
// 引入 `tinybmp` 库，处理 BMP 图像格式
use tinybmp::Bmp;
// 引入 `volatile` 库中的 `Volatile` 类型，用于对内存中可能随时改变的数据进行读写操作
use volatile::Volatile;
// 引入 x86_64 架构相关的分页模块和类型，包括帧分配器、偏移页表、页面以及虚拟地址 (`VirtAddr`) 类型
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable, Page, Size4KiB};
use x86_64::VirtAddr;

// 引入自定义模块中的函数 `qemu_print`, 用于打印调试信息到 QEMU 控制台
use crate::qemu::qemu_print;

pub mod vbe;
pub mod font;

// 定义一个表示像素数据的结构体，包含红色、绿色和蓝色分量。使用C语言风格布局保证字段顺序一致性，并实现一些常用的trait如Debug、Clone等，以方便使用和调试

// 相关配置 
// 定义屏幕宽度和高度为800像素
const WIDTH: usize = 800;
const HEIGHT: usize = 800;

// 相关数据结构

// 提交到内存的像素
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
}

// 定义一个屏幕缓冲区结构体，它是一个二维数组，每个元素都是具有易变特性的像素。这里使用透明属性使得Buffer与其内部数组具有相同布局
#[repr(transparent)]
pub struct Buffer {
    chars: [[Volatile<Pixel>; WIDTH]; HEIGHT],
}

// 定义显示器结构体，它包含了一个缓冲区对象.
pub struct Writer(&'static mut Buffer);

// 相关常量
// 使用lazy_static宏创建一个全局静态缓冲区对象，并将其包装在互斥锁中以确保线程安全。通过不安全代码将虚拟地址转换为指向缓冲区的指针
lazy_static! {
    pub static ref GD: Mutex<Writer> = {
        Mutex::new(Writer(unsafe {&mut *(Page::<Size4KiB>::containing_address(VirtAddr::new(0xC000_0000)).start_address().as_mut_ptr() as *mut Buffer) }))
    };
}

// 定义进入宽屏模式的方法，通过调用外部模块vbe的方法来实现具体操作
pub fn enter_wide_mode(
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>) {
    unsafe { vbe::bga_enter_wide(mapper, frame_allocator); }
}

// 实现显示器结构体的方法：
// - display_pixel：直接根据RGB颜色值写像素；因为处于性能关键路径，不做边界检查。
// - display_pixel_rgb888：根据RGB888颜色值写像素，同样不做边界检查，并且通过BUFFER全局变量获取实际显示缓冲区

impl Writer {
    // 写像素
    // color是一个按照_RGB格式给出颜色的数字
    // 因为这个函数在关键路径上，所以就不检查边界了
    pub unsafe fn display_pixel(&mut self, x: usize, y: usize, color: u32) {
        self.0.chars[x][y].write(Pixel {
            r: color as u8,
            g: (color >> 8) as u8,
            b: (color >> 16) as u8,
        });
    }

    // 写像素
    // color是RGB888
    // 因为这个函数在关键路径上，所以就不检查边界了
    pub unsafe fn display_pixel_rgb888(&mut self, x: usize, y: usize, color: Rgb888) {
        self.0.chars[x][y].write(Pixel {
            b: color.r(),
            g: color.g(),
            r: color.b(),
        });
    }

    // 定义矩形绘制方法：
    //  - 根据输入参数计算结束位置；
    //  - 打印调试信息；
    //  - 循环遍历每个点并调用display_pixel方法绘制矩形.
    pub fn display_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let x_end = min(x + h, HEIGHT);
        let y_end = min(y + w, WIDTH);
        qemu_print(format!("{},{},{},{}\n", x, y, x_end, y_end).as_str());
        for i in x..x_end {
            for j in y..y_end {
                unsafe { self.display_pixel(i, j, color); };
            }
        }
    }

    // 测试函数：画一幅图
    // 定义图片展示方法：
    // - 从BMP数据解析得到图像对象；
    // - 遍历每个像素并调用display_pixel_rgb88方法绘制图像；
    // - 如果解析失败，则打印错误信息
    pub fn display_img(&mut self, x: usize, y: usize, bmp_data: &[u8]) {
        match Bmp::<Rgb888>::from_slice(bmp_data) {
            Ok(bmp) => {
                for Pixel(position, color) in bmp.pixels() {
                    unsafe { self.display_pixel_rgb888(x + position.y as usize, y + position.x as usize, color); };
                }
            }
            Err(error) => {
                qemu_print(format!("{:?}\n", error).as_str());
            }
        }
    }
}

// 本代码片段主要完成以下功能：

// 1. **基本设置**：包括导入必要库和模块，定义屏幕尺寸。
// 2. **核心数据结构**：
//    - Pixel：表示单个像素。
//    - Buffer：表示整个屏幕，由二维数组组成，每个元素是带有易变特性的Pixel。
//    - Writer：封装了Buffer，实现了一些操作方法。
// 3. **全局变量**：使用LazyStatic创建全局静态缓冲区对象，并确保线程安全。
// 4. **模式切换**：提供进入宽屏模式的方法，通过外部模块实现具体操作.
// 5. **显示逻辑**：
//    - 提供直接根据RGB或RGB88颜色写单个像素的方法.
//    - 提供绘制矩形和展示图片的方法，通过遍历指定区域或图象数据逐点调用上述单点绘画接口来实现.
