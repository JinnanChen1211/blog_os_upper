// 引入Rust的格式化模块，用于输出显示
use core::fmt;
// 引入写接口，使得可以使用write!宏来打印
use core::fmt::Write;
// 引入`Volatile`类型封装内存，确保每次修改都是直接对硬件的
use volatile::Volatile;

// VGA标准颜色
// 允许未使用代码不被警告
#[allow(dead_code)]
// 为枚举派生Debug、Clone、Copy等trait，方便调试和值复制
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// 表示每个枚举值将以u8（一个字节）形式存储
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// 表示在内存中该结构体会像其单一字段那样布局，有助于避免布局问题和提高性能
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, bcakground: Color) -> ColorCode {
        // 创建一个新的ColorCode实例。前景色放在低4位，背景色放在高4位，并转换为u8类型进行按位运算后返回
        ColorCode((bcakground as u8) << 4 | (foreground as u8))
    }
}

// 提交到内存中的VGA字符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// 设置此结构体在内存中的表示应遵循C语言的排列方式
// 确保其具有与C语言相同的内存布局；这通常意味着字段会按照它们声明时候顺序紧密排列。
#[repr(C)]
struct ScreenChar {
    // 存储单个字符使用的ASCII码（1个字节)
    ascii_character: u8,
    // 存储包含前景色和背景色信息（合起来也是1个字节）的ColorCode结构体实例
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
// 定义Tab键对应空格数
const TAB_SIZE: usize = 4;

// 表示 VGA 文本模式下屏幕的整个字符缓冲区
#[repr(transparent)]
struct Buffer {
    // 使用二维数组代表屏幕每个位置的字符信息，并包裹在Volatile内以防止编译器优化掉直接写入操作
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// 输出器
pub struct Writer {
    row_position: usize,
    column_position: usize,
    color_code: ColorCode,
    // 静态生命周期引用当前VGA缓冲区 允许整个程序运行期间可变地访问这个Buffer
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            0x08 => self.backspace(),
            b'\t' => self.horizontal_tab(),
            b'\n' => self.new_line(),
            b'\r' => self.carriage_return(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line()
                }
                let row = self.row_position.clone();
                let col = self.column_position.clone();
                let color_code = self.color_code.clone();
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });

                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' | b'\r' | b'\t' | 0x08 => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row.clone()][col].write(blank);
        }
    }

    fn new_line(&mut self) {
        self.row_position += 1;
        self.column_position = 0;

        if self.row_position >= BUFFER_HEIGHT {
            // 向上滚屏
            for row in 0..BUFFER_HEIGHT - 1 {
                for col in 0..BUFFER_WIDTH {
                    self.buffer.chars[row.clone()][col.clone()].write(self.buffer.chars[row.clone() + 1][col.clone()].read());
                }
            }
            self.clear_row(BUFFER_HEIGHT - 1);
        }
    }

    fn backspace(&mut self) {
        if self.column_position > 0 {
            self.column_position -= 1;
        }
    }

    fn carriage_return(&mut self) {
        self.column_position = 0;
    }

    fn horizontal_tab(&mut self) {
        self.column_position += TAB_SIZE - (self.column_position.clone() % TAB_SIZE);
        if self.column_position >= BUFFER_WIDTH {
            self.new_line();
        }
    }

}

pub fn print_something() {
    let mut writer = Writer {
        row_position: 0,
        column_position: 0,
        color_code: ColorCode::new(Color::LightCyan, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    };

    write!(writer, "Every smallest dream matters.\n\n").unwrap();
    write!(writer, "\t----Hello World From cjn's Operating System\n").unwrap();
    write!(writer, "\t\t\t\t\t\t\t\t2024.08.01").unwrap();
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        self.write_string(s);
        Ok(())
    }
}
