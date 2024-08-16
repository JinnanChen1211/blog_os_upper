
#![no_std] // 不链接Rust标准库
#![no_main] // 禁用所有Rust层级的入口点
#![feature(abi_x86_interrupt)]

extern crate alloc;
use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};


use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};
use x86_64::structures::paging::Page;
use x86_64::VirtAddr;
use cjn_os::{allocator, println};
use cjn_os::graphic::{enter_wide_mode, GD};
use cjn_os::graphic::font::test_font;
use cjn_os::memory::graphic_support::create_graphic_memory_mapping;
use cjn_os::qemu::qemu_print;
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

    use cjn_os::memory::BootInfoFrameAllocator;
    use cjn_os::allocator;

    // 使用来自于`boot_info.physical_memory_offset`的值创建了一个新的 `VirtAddr`(虚拟内存地址)实例。这个偏移量被用于在物理和虚拟地址之间进行转换
    let phys_mem_offset: VirtAddr = VirtAddr::new(boot_info.physical_memory_offset.clone());
    // 调用一个不安全函数 `cjn_os::memory::init` 并传入 `phys_mem_offset`，进行物理内存到虚拟内存的映射初始化，将返回值赋予变量 `mapper`。
    // - **原因**: 初始化内存映射器，用于将一些虚拟地址映射到物理地址上，通常在操作系统启动时设置
    // let mut mapper = unsafe{cjn_os::memory::init(phys_mem_offset)};
    let mut mapper = unsafe { cjn_os::memory::init(phys_mem_offset) };
    // 调用另一个不安全函数 `BootInfoFrameAllocator::init`，传入 boot info（引导信息）中的内存图（memory map），并生成帧分配器实例。
    // - **原因**: 帧分配器负责管理物理内存帧，它可以提供新的帧供使用或回收不再需要的帧
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    // 调用 `allocator::init_heap` 方法传入前面初始化好的内存映射器和帧分配器来初始化堆空间，如失败则输出错误信息"Heap initialization failed"。
    // - **原因**: 在无操作系统环境中，手动初始化堆非常重要，以便后续动态分配资源
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed");
    // map an unused page
    // 映射一页未使用的页面。这里通过传入虚拟地址 0 来创建代表此页面变量 `page`。
    // - **原因**: 映射特定虚拟地址对应的一页。这通常用于测试或系统启动过程中对某些固定位置进行特殊处理
    // let page = Page::containing_address(VirtAddr::new(0));
    
    // 调用了 `cjn_os` 模块中的 `create_example_mapping` 函数，将页面 `page` 映射到物理内存。它传入了虚拟地址页面对象 `page`、内存映射器 `mapper` 和帧分配器 `frame_allocator`。
    //    - **原因**: 这是为了在无操作系统环境中手动管理内存，确保特定虚拟地址被正确映射到物理内存。
    // cjn_os::memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

        // let heap_value = Box::new(831);
    // println!("heap_value is at {:p}", heap_value);
    //
    // let mut vec = Vec::new();
    // for i in 0..500 {
    //     vec.push(i)
    // }
    // println!("vec at {:p}", vec.as_slice());
    //
    // let reference_counted = Rc::new(vec![1, 2, 3]);
    // let cloned_reference = reference_counted.clone();
    // println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    // core::mem::drop(reference_counted);
    // println!("reference count is {} now", Rc::strong_count(&cloned_reference));

    qemu_print("The OS is leaving VGA now...\n");

    enter_wide_mode(&mut mapper, &mut frame_allocator);

    GD.lock().display_rect(0, 0, 800, 600, 0xFFFFFF);

    // // write the string `New!` to the screen through the new mapping
    // // - 首先，获取页面起始地址并转换为可变指针类型：`let page_ptr: *mut u64 = page.start_address().as_mut_ptr();`
    // // - **原因**: 为了能够直接操作该虚拟地址对应的物理内存。
    // // - 然后，通过偏移量 400 来写入数据：`unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};`
    // // - **原因**: 使用这个偏移量和具体的数据（这里是十六进制数）将字符串 "New!" 写入屏幕缓冲区。
    // let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    // unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};
    // // 创建了一个包含整数 41 的 Box 对象，并将其分配在堆上。
    //  // - **原因**: 测试堆的初始化是否成功，以及后续动态分配功能是否正常工作
    // let _x = Box::new(41);
    // println!("{:?}",x);

    let lpld = include_bytes!("../assets/91527085_p0.bmp");
    let cjn_os = include_bytes!("../assets/cjn-os.bmp");
    //GD.lock().display_img(0, 0, lpld);
    GD.lock().display_img(400, 300, cjn_os);
    test_font();
    // 进入无限循环防止 `_start` 函数,返回也确保内核不会意外退出到未定义行为状态中去
    cjn_os::hlt_loop();
}

// 总结： 
// 这段代码的主要目的是在操作系统初始化期间设置内存管理，包括初始化堆空间和执行具体的内存页面映射。以下是主要步骤：

// 1. **初始化内存映射器**：设置并生成一个用于物理地址到虚拟地址转换的映射器 `mapper`。
// 2. **初始化帧分配器**：通过引导信息中的内存图，生成一个新帧分配器 `frame_allocator` 来管理物理内存帧。
// 3. **初始化堆空间**：使用上述两个组件来完成堆空间的初始化，以支持动态内存分配。
// 4. **映射未使用页**：创建一个表示特定（此处为0）虚拟地址的一页 `page` 并进行具体实现映射。
// 5. **示例写操作**：通过新建立的页面映射执行具体写入操作，如向屏幕位置写入特定字符串内容测试其正确性。

// 整个过程确保了在去除了完整标准库环境下，依然能够顺利进行基本的动态内存管理和页面映射，适用于嵌入式和裸机开发等场景
