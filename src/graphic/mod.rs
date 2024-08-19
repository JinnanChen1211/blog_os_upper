// 引入 `alloc` 库中的 `format` 宏，用于格式化字符串
use alloc::{format, vec};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
// 引入 `core` 库中的 `min` 函数，用于计算两个值的较小值
use core::cmp::min;
// 引入 `embedded_graphics` 库中的颜色类型 `Rgb888` 和预导出的所有内容（prelude），以及另一个颜色类型 `Bgr888`
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
// 引入 `lazy_static` 宏，用于声明静态变量并进行延迟初始化
use lazy_static::lazy_static;
use rusttype::{point, Rect, ScaledGlyph};
use spin::{Mutex, RwLock};
use tinybmp::{Bmp, ChannelMasks, RawBmp, RawPixel};
// 引入 `volatile` 库中的 `Volatile` 类型，用于对内存中可能随时改变的数据进行读写操作
use volatile::Volatile;
use x86_64::instructions::interrupts;
// 引入 x86_64 架构相关的分页模块和类型，包括帧分配器、偏移页表、页面以及虚拟地址 (`VirtAddr`) 类型
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable, Page, Size4KiB};
use x86_64::VirtAddr;

use crate::graphic::color::alpha_mix;
use crate::graphic::font::get_font;
use crate::graphic::text::TEXT_WRITER;
use crate::io::qemu::qemu_print;
use crate::io::VIDEO_MODE;
use crate::rgb888;

pub mod vbe;
pub mod font;
pub mod text;
pub mod color;

// 定义一个表示像素数据的结构体，包含红色、绿色和蓝色分量。使用C语言风格布局保证字段顺序一致性，并实现一些常用的trait如Debug、Clone等，以方便使用和调试

// 相关配置 
// 定义屏幕宽度和高度为800像素
pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 600;

// 定义一个屏幕缓冲区结构体，它是一个二维数组，每个元素都是具有易变特性的像素。这里使用透明属性使得Buffer与其内部数组具有相同布局
#[repr(transparent)]
pub struct Buffer {
    chars: [[Volatile<Rgb888>; WIDTH]; HEIGHT],
}

// 定义显示器结构体，它包含了一个缓冲区对象.
pub struct PhysicalWriter(&'static mut Buffer);

#[derive(Clone, Debug)]
pub struct Writer {
    pub data: Vec<Vec<(Rgb888, bool)>>,
    pub enable: bool,
}

// 使用lazy_static宏创建一个全局静态缓冲区对象，并将其包装在互斥锁中以确保线程安全。通过不安全代码将虚拟地址转换为指向缓冲区的指针
lazy_static! {
    // 这个是最底层的显存
    pub static ref GD: Mutex<PhysicalWriter> = {
        Mutex::new(PhysicalWriter(unsafe {&mut *(Page::<Size4KiB>::containing_address(VirtAddr::new(0xC000_0000)).start_address().as_mut_ptr() as *mut Buffer) }))
    };

    // 多层叠加显示
    pub static ref GL: RwLock<Vec<Mutex<Writer>>> = {
        let mut v:Vec<Mutex<Writer>> = vec![];
        v.reserve(5);
        for _ in 0..5{
            v.push(Mutex::new(Writer::new()));
        }
        RwLock::new(v)
    };
}

// 定义进入宽屏模式的方法，通过调用外部模块vbe的方法来实现具体操作
pub fn enter_wide_mode(
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>) {
    unsafe { vbe::bga_enter_wide(mapper, frame_allocator); 
    }
    VIDEO_MODE.lock().set_graphic();
}

// 实现显示器结构体的方法：
// - display_pixel：直接根据RGB颜色值写像素；因为处于性能关键路径，不做边界检查。
// - display_pixel_rgb888：根据RGB888颜色值写像素，同样不做边界检查，并且通过BUFFER全局变量获取实际显示缓冲区

impl PhysicalWriter {
    // 写像素
    // color是一个按照_RGB格式给出颜色的数字
    // 因为这个函数在关键路径上，所以就不检查边界了
    pub unsafe fn display_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        self.0.chars[x][y].write(color);
    }

    pub fn display_pixel_safe(&mut self, x: usize, y: usize, color: Rgb888) {
        if x < HEIGHT && y < WIDTH {
            self.0.chars[x][y].write(color);
        }
    }

    // 定义矩形绘制方法：
    //  - 根据输入参数计算结束位置；
    //  - 打印调试信息；
    //  - 循环遍历每个点并调用display_pixel方法绘制矩形.
    pub fn display_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Rgb888) {
        let x_end = min(x + h, HEIGHT);
        let y_end = min(y + w, WIDTH);
        qemu_print(format!("{},{},{},{}\n", x, y, x_end, y_end).as_str());
        for i in x..x_end {
            for j in y..y_end {
                unsafe { self.display_pixel(i, j, color); };
            }
        }
    }

    // 定义图片展示方法：
    // - 从BMP数据解析得到图像对象；
    // - 遍历每个像素并调用display_pixel_rgb88方法绘制图像；
    // - 如果解析失败，则打印错误信息
    pub fn display_img(&mut self, x: usize, y: usize, bmp_data: &[u8]) {
        match Bmp::<Rgb888>::from_slice(bmp_data) {
            Ok(bmp) => {
                for Pixel(position, color) in bmp.pixels() {
                    unsafe { self.display_pixel(x + position.y as usize, y + position.x as usize, color); };
                }
            }
            Err(error) => {
                qemu_print(format!("{:?}\n", error).as_str());
            }
        }
    }
    pub fn display_font(&mut self, glyph: ScaledGlyph, x_pos: usize, y_pos: usize, size: f32, line_height: usize, fg_color: Rgb888, bg_color: Rgb888) {
        let bbox = glyph.exact_bounding_box().unwrap_or(Rect {
            min: point(0.0, 0.0),
            max: point(size, size),
        });

        let x_offset = (line_height as f32 + bbox.min.y) as usize;
        //qemu_print(format!("{:?},{:?},{:?}\n",ch,bbox,x_offset).as_str());
        let glyph = glyph.positioned(point(0.0, 0.0));
        glyph.draw(|y, x, v| {
            let (color, _) = alpha_mix(fg_color, v, bg_color, 1.0);
            self.display_pixel_safe(x_offset + x_pos + x as usize, y_pos + y as usize + bbox.min.x as usize, color);
        })
    }

    /// 敬请注意：此方法不检查换行
    pub unsafe fn display_font_string(&mut self, s: &str, x_pos: usize, y_pos: usize, size: f32, line_height: usize, fg_color: Rgb888, bg_color: Rgb888) {
        let mut y_pos = y_pos;
        for ch in s.chars() {
            if y_pos >= WIDTH { return; }
            let (glyph, hm) = get_font(ch, size);
            self.display_font(glyph, x_pos, y_pos, size, line_height, fg_color, bg_color);
            y_pos += hm.advance_width as usize + 1usize;
        }
    }
}

const DEFAULT_RGB888: Rgb888 = Rgb888::new(0, 0, 0);

impl Writer {
    pub fn new() -> Self {
        Self {
            data: vec![vec![(DEFAULT_RGB888, false); WIDTH]; HEIGHT],
            enable: false,
        }
    }

    /// 写像素
    /// color是RGB888
    ///
    /// 因为这个函数在关键路径上，所以就不检查边界了
    pub unsafe fn display_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        self.data[x][y] = (color, true);
    }

    pub fn display_pixel_safe(&mut self, x: usize, y: usize, color: Rgb888) {
        if x < HEIGHT && y < WIDTH {
            self.data[x][y] = (color, true);
        }
    }

    pub fn display_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Rgb888) {
        let x_end = min(x + h, HEIGHT);
        let y_end = min(y + w, WIDTH);
        for i in x..x_end {
            for j in y..y_end {
                self.data[i][j] = (color, true);
            }
        }
    }

    pub fn display_img(&mut self, x: usize, y: usize, bmp_data: &[u8]) {
        match Bmp::<Rgb888>::from_slice(bmp_data) {
            Ok(bmp) => {
                for Pixel(position, color) in bmp.pixels() {
                    self.data[x + position.y as usize][y + position.x as usize] = (color, true);                }
            }
            Err(error) => {
                qemu_print(format!("{:?}\n", error).as_str());
            }
        }
    }

    pub fn display_img_32rgba(&mut self, x: usize, y: usize, bmp_data: &[u8]) {
        match RawBmp::from_slice(bmp_data) {
            Ok(bmp) => {
                let cm = match bmp.header().channel_masks {
                    None => {
                        ChannelMasks {
                            blue: 0x000000FF,
                            green: 0x0000FF00,
                            red: 0x00FF0000,
                            alpha: 0xFF000000,
                        }
                    },
                    Some(cm) => cm
                };
                let (mut rr, mut br, mut gr, mut ar) = (0, 0, 0, 0);
                let mut rm = cm.red;
                while rm & 1 == 0 {
                    rr += 1;
                    rm >>= 1;
                }
                let mut gm = cm.green;
                while gm & 1 == 0 {
                    gr += 1;
                    gm >>= 1;
                }
                let mut bm = cm.blue;
                while bm & 1 == 0 {
                    br += 1;
                    bm >>= 1;
                }
                let mut am = cm.alpha;
                while am & 1 == 0 {
                    ar += 1;
                    am >>= 1;
                }
                let asize = (cm.alpha >> ar) as f32;

                for RawPixel { position, color } in bmp.pixels() {
                    let rgb_color = Rgb888::new(((color & cm.red) >> rr) as u8, ((color & cm.green) >> gr) as u8, ((color & cm.blue) >> br) as u8);
                    let alpha = ((color & cm.alpha) >> ar) as f32 / asize;
                    //qemu_print(format!("{:?},{:?}", rgb_color, alpha).as_str());
                    if alpha > 0.5 {
                        self.display_pixel_safe(x + position.y as usize, y + position.x as usize, rgb_color);
                    }
                }
            }
            Err(error) => {
                qemu_print(format!("{:?}\n", error).as_str());
            }
        }
    }

    pub fn display_font(&mut self, glyph: ScaledGlyph, x_pos: usize, y_pos: usize, size: f32, line_height: usize, color: Rgb888) {
        let bbox = glyph.exact_bounding_box().unwrap_or(Rect {
            min: point(0.0, 0.0),
            max: point(size, size),
        });

        let x_offset = (line_height as f32 + bbox.min.y) as usize;
        //qemu_print(format!("{:?},{:?},{:?}\n",ch,bbox,x_offset).as_str());

        let glyph = glyph.positioned(point(0.0, 0.0));
        glyph.draw(|y, x, v| {
            if v > 0.5 {
                self.display_pixel_safe(x_offset + x_pos + x as usize, y_pos + y as usize + bbox.min.x as usize, color);
            }
        });
    }

    /// 敬请注意：此方法不检查换行
    pub unsafe fn display_font_string(&mut self, s: &str, x_pos: usize, y_pos: usize, size: f32, line_height: usize, color: Rgb888) {
        let mut y_pos = y_pos;
        for ch in s.chars() {
            if y_pos >= WIDTH { return; }
            let (glyph, hm) = get_font(ch, size);
            self.display_font(glyph, x_pos, y_pos, size, line_height, color);
            y_pos += hm.advance_width as usize + 1usize;
        }
    }

    ///将图像整体移动
    pub fn move_to(&mut self, dx: i32, dy: i32) {
        let x_iter: Box<dyn Iterator<Item=usize>> = if dx > 0 {
            Box::new(0..HEIGHT)
        } else {
            Box::new((0..HEIGHT).rev())
        };
        for i in x_iter {
            let y_iter: Box<dyn Iterator<Item=usize>> = if dy > 0 {
                Box::new(0..WIDTH)
            } else {
                Box::new((0..WIDTH).rev())
            };
            for j in y_iter {
                if ((i as i32 - dx) as usize) < HEIGHT && ((j as i32 - dy) as usize) < WIDTH {
                    self.data[i][j] = self.data[(i as i32 - dx) as usize][(j as i32 - dy) as usize];
                } else {
                    self.data[i][j] = (DEFAULT_RGB888, false);
                }
            }
        }
    }
}

impl PhysicalWriter {
    pub fn render(&mut self, sx: usize, sy: usize, ex: usize, ey: usize) {
        //qemu_print(format!("Start Render... Now is {:?}\n", TIME.lock()).as_str());
        if sx < HEIGHT && sy < WIDTH && ex <= HEIGHT && ey <= WIDTH {
            if GL.read().len() == 0 { return; }
            let p_lock = GL.read();
            let lock = p_lock[p_lock.len() - 1].lock();
            let mut graph: Box<Vec<Vec<(Rgb888, bool)>>> = Box::new(lock.data.clone());
            drop(lock);
            for layer in (1..p_lock.len() - 1).rev() {
                let lock = p_lock[layer].lock();
                if !lock.enable { continue }
                let tomix = &lock.data;
                for x in sx..ex {
                    for y in sy..ey {
                        if !graph[x][y].1 && tomix[x][y].1 {
                            graph[x][y] = tomix[x][y]
                        }
                    }
                }
            }
            let tomix = &p_lock[0].lock().data;
            for x in sx..ex {
                for y in sy..ey {
                    graph[x][y].0 = if graph[x][y].1 { graph[x][y].0 } else { tomix[x][y].0 };
                }
            }
            for x in sx..ex {
                for y in sy..ey {
                    self.0.chars[x][y].write(graph[x][y].0);
                }
            }
        }
        //qemu_print(format!("Finish Render... Now is {:?}\n", TIME.lock()).as_str());
    }
}

pub fn test_img() {
    GD.lock().display_rect(0, 0, 800, 600, rgb888!(0xFFFFFFu32));

    unimplemented!("请为我解除封印");
    // let lpld = include_bytes!("../assets/91527085_p0.bmp");
    // let cinea_os = include_bytes!("../assets/cinea-os.bmp");
    // GD.lock().display_img(0, 0, lpld);
    // GD.lock().display_img(400, 300, cinea_os);
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    // 防止死锁
    interrupts::without_interrupts(|| {
        TEXT_WRITER.lock().write_fmt(args).unwrap();
    })
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
