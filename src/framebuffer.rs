use alloc::vec;
use alloc::vec::Vec;
use core::borrow::Borrow;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use log::info;
use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput};
use uefi_services::system_table;

pub struct Framebuffer<'a> {
    buffer: Vec<BltPixel>,
    width: u32,
    height: u32,
    gop: &'a mut GraphicsOutput<'a>,
}

impl Borrow<[BltPixel]> for Framebuffer<'_> {
    fn borrow(&self) -> &[BltPixel] {
        &self.buffer
    }
}

impl DrawTarget<Rgb888> for Framebuffer<'_> {
    type Error = ();

    fn draw_pixel(&mut self, item: Pixel<Rgb888>) -> Result<(), Self::Error> {
        let Pixel(coord, color) = item;
        if coord.x < self.width as i32 && coord.y < self.height as i32 {
            self.buffer[(coord.y * self.width as i32 + coord.x) as usize] =
                BltPixel::new(color.r(), color.g(), color.b());
        }
        Ok(())
    }

    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    fn clear(&mut self, color: Rgb888) -> Result<(), Self::Error> {
        self.buffer = vec![
            BltPixel::new(color.r(), color.g(), color.b());
            (self.width * (self.height + 1)) as usize
        ];
        Ok(())
    }
}

impl Framebuffer<'_> {
    pub fn new(width: u32, height: u32) -> Self {
        let protocol = unsafe {
            system_table()
                .as_ref()
                .boot_services()
                .locate_protocol::<GraphicsOutput>()
                .unwrap()
                .unwrap()
        };

        let gop = unsafe { &mut *protocol.get() };
        info!("Created framebuffer: {:?}", gop.current_mode_info());
        Self {
            buffer: vec![BltPixel::new(0, 0, 0); (width * (height + 1)) as usize],
            width,
            height,
            gop,
        }
    }

    pub fn draw_buffer(&mut self) {
        self.gop
            .blt(BltOp::BufferToVideo {
                buffer: &self.buffer.borrow(),
                src: BltRegion::Full,
                dest: (0, 0),
                dims: (800, 600),
            })
            .unwrap()
            .unwrap();
    }
}
