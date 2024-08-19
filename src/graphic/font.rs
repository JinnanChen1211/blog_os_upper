use lazy_static::lazy_static;
use rusttype::{Font, HMetrics, Scale, ScaledGlyph};

const FONT_DATA: &[u8] = include_bytes!("../../assets/VonwaonBitmap-16px.ttf");

// 使用 `lazy_static!` 宏定义一个静态变量 `FONT`, 初始化为从字节数组中加载的字体对象
lazy_static! {
    pub(super) static ref FONT: Font<'static> = Font::try_from_bytes(FONT_DATA).unwrap();
}

pub fn get_font(ch: char, size: f32) -> (ScaledGlyph<'static>, HMetrics){
    let scale = Scale::uniform(size);
    let glyph_id = FONT.glyph(ch);
    let glyph = glyph_id.scaled(scale);
    let h_metrics = glyph.h_metrics();
    (glyph,h_metrics)
}

// 步骤:
// 1. 导入`alloc`库中的`format`和`ToString`，用于字符串格式化。
// 2. 导入`lazy_static`宏，用于定义静态变量。
// 3. 导入`rusttype`库中的字体处理相关模块，包括字体、点和缩放比例。
// 4. 从项目的其他模块导入自定义的图形显示（GD）和QEMU打印函数（qemu_print）。
// 5. 定义常量 `FONT_DATA`，包含字体文件的数据。
// 6. 使用 `lazy_static!` 宏定义一个静态变量 `FONT`, 初始化为从字节数组中加载的字体对象
// 7. 定义函数 `test_font()`。
// 8. 定义字符串样本 `sample="Test只因你太美"`。
// 9. 初始化浮点数 `fx=0.0f32`, 用于记录当前绘制位置的横向偏移。
// 10-11. 遍历字符串中的每个字符及其索引 `(i,ch)`：
//    - 设置缩放比例为16.
//    - 获取字符对应的字形ID并进行缩放处理 (`glyph.scaled(scale)`).
// 12-14 获取水平度量信息 (`h_metrics`) 并打印字符及其度量信息到 QEMU 控制台 (`qemu_print`)。
// 15-17 计算当前字符的位置偏移量 (`offset_x`, `offset_y`) 和具体位置（point）。
// 18 将字形定位到计算的位置上 (`glyph.positioned(position)`).
// 19 锁定图形显示设备 (`gd=GD.lock()`).
// 20-25 绘制字形，将每个像素值转换为颜色并显示到屏幕上。这里使用了闭包将像素值传递给显示函数。
// 26 更新横向偏移量以便下一个字符紧跟前一个字符之后绘制。

// ### 总结
// 这段代码实现了从TTF字体文件加载字体数据，遍历字符串中的每个字符，根据指定缩放比例生成字形，并通过图形设备在屏幕上绘制这些字形。还会将每个字符和它的度量信息输出到QEMU控制台。
