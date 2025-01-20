use bootloader_api::info::{FrameBuffer, PixelFormat};

use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{Rgb888, RgbColor},
};
use lazy_static::lazy_static;
use spin::Mutex;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

    /// Sets a pixel in the given `framebuffer` at position `position` to
    /// `color`.
    ///
    /// The color is written to the framebuffer in the pixel format of the
    /// framebuffer. The pixel formats `Rgb`, `Bgr`, and `U8` are supported.
    ///
    /// # Panics
    ///
    /// Panics if the pixel format of the framebuffer is unknown.
    ///
pub fn set_pixel_in(framebuffer: &mut FrameBuffer, position: Position, color: Color) {
    let info = framebuffer.info();

    // calculate offset to first byte of pixel
    let byte_offset = {
        // use stride to calculate pixel offset of target line
        let line_offset = position.y * info.stride;
        // add x position to get the absolute pixel offset in buffer
        let pixel_offset = line_offset + position.x;
        // convert to byte offset
        pixel_offset * info.bytes_per_pixel
    };

    // set pixel based on color format
    let pixel_buffer = &mut framebuffer.buffer_mut()[byte_offset..];
    match info.pixel_format {
        PixelFormat::Rgb => {
            pixel_buffer[0] = color.red;
            pixel_buffer[1] = color.green;
            pixel_buffer[2] = color.blue;
        }
        PixelFormat::Bgr => {
            pixel_buffer[0] = color.blue;
            pixel_buffer[1] = color.green;
            pixel_buffer[2] = color.red;
        }
        PixelFormat::U8 => {
            // use a simple average-based grayscale transform
            let gray = color.red / 3 + color.green / 3 + color.blue / 3;
            pixel_buffer[0] = gray;
        }
        other => panic!("unknown pixel format {other:?}"),
    }
}

pub struct Display<'f> {
    framebuffer: &'f mut FrameBuffer,
}

impl<'f> Display<'f> {
    pub fn new(framebuffer: &'f mut FrameBuffer) -> Display {
        Display { framebuffer }
    }

    fn draw_pixel(&mut self, Pixel(coordinates, color): Pixel<Rgb888>) {
        // ignore any out of bounds pixels
        let (width, height) = {
            let info = self.framebuffer.info();

            (info.width, info.height)
        };

        let (x, y) = {
            let c: (i32, i32) = coordinates.into();
            (c.0 as usize, c.1 as usize)
        };

        if (0..width).contains(&x) && (0..height).contains(&y) {
            let color = Color { red: color.r(), green: color.g(), blue: color.b() };

            set_pixel_in(self.framebuffer, Position { x, y }, color);
        }
    }
}

impl<'f> DrawTarget for Display<'f> {
    type Color = Rgb888;

    /// Drawing operations can never fail.
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels.into_iter() {
            self.draw_pixel(pixel);
        }

        Ok(())
    }
}

impl<'f> OriginDimensions for Display<'f> {
    fn size(&self) -> Size {
        let info = self.framebuffer.info();

        Size::new(info.width as u32, info.height as u32)
    }
}

use noto_sans_mono_bitmap::{
    get_raster, get_raster_width, FontWeight, RasterHeight, RasterizedChar,
};

// Available heights are:
// - RasterHeight::Size6
// - RasterHeight::Size7
// - RasterHeight::Size8
// - RasterHeight::Size9
// - RasterHeight::Size10
// - RasterHeight::Size11
// - RasterHeight::Size12
// - RasterHeight::Size13
// - RasterHeight::Size14
// - RasterHeight::Size15
// - RasterHeight::Size16
// - RasterHeight::Size17
// - RasterHeight::Size18
// - RasterHeight::Size19
// - RasterHeight::Size20
// - RasterHeight::Size21
// - RasterHeight::Size22
// - RasterHeight::Size23
// - RasterHeight::Size24

pub struct Writer {
    column_position: usize,
    row_position: usize,
    color_code: Color,
    buffer: &'static mut FrameBuffer,
    font_height: RasterHeight,
    font_weight: FontWeight,
}

impl Writer {
    pub fn new(
        buffer: &'static mut FrameBuffer,
        color: Color,
        height: RasterHeight,
        weight: FontWeight,
    ) -> Self {
        Self {
            column_position: 0,
            row_position: 0,
            color_code: color,
            buffer,
            font_height: height,
            font_weight: weight,
        }
    }

    fn char_width(&self) -> usize {
        get_raster_width(self.font_weight, self.font_height)
    }

    fn char_height(&self) -> usize {
        self.font_height.val()
    }

    pub fn clear_screen_with_color(&mut self, color: Color) {
        let info = self.buffer.info();

        for y in 0..info.height {
            for x in 0..info.width {
                set_pixel_in(
                    self.buffer,
                    Position { x, y },
                    color
                );
            }
        }

        self.column_position = 0;
        self.row_position = 0;
    }
    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.new_line(),
            '\r' => self.carriage_return(),
            c => {
                let info = self.buffer.info();
                if self.column_position >= (info.width / self.char_width()) {
                    self.new_line();
                }
                if self.row_position >= (info.height / self.char_height()) {
                    self.scroll();
                }
                self.write_rendered_char(get_raster(c, self.font_weight, self.font_height).unwrap());
                self.column_position += 1;
            }
        }
    }

    fn write_rendered_char(&mut self, rendered_char: RasterizedChar) {
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, &intensity) in row.iter().enumerate() {
                if intensity > 0 {
                    let scaled_intensity = (intensity as f32) / 255.0;
                    let color = Color {
                        red: (self.color_code.red as f32 * scaled_intensity) as u8,
                        green: (self.color_code.green as f32 * scaled_intensity) as u8,
                        blue: (self.color_code.blue as f32 * scaled_intensity) as u8,
                    };
                    let pos = Position {
                        x: self.column_position * self.char_width() + x,
                        y: self.row_position * self.char_height() + y,
                    };
                    set_pixel_in(self.buffer, pos, color);
                }
            }
        }
    }

    fn new_line(&mut self) {
        self.row_position += 1;
        self.carriage_return();
    }

    fn carriage_return(&mut self) {
        self.column_position = 0;
    }

    fn scroll(&mut self) {
        let info = self.buffer.info();
        let bytes_per_line = info.stride * info.bytes_per_pixel;
        // Get the char_height before borrowing buffer
        let char_height = self.char_height();

        let buffer = self.buffer.buffer_mut();

        // Move all lines up by one character height
        for line in 0..(info.height - char_height) {
            let src_offset = (line + char_height) * bytes_per_line;
            let dst_offset = line * bytes_per_line;
            buffer.copy_within(
                src_offset..(src_offset + bytes_per_line),
                dst_offset,
            );
        }

        // Clear the last line
        let last_line_offset = (info.height - char_height) * bytes_per_line;
        for y in 0..char_height {
            for x in 0..info.width {
                let pos = Position {
                    x,
                    y: info.height - char_height + y,
                };
                set_pixel_in(self.buffer, pos, Color { red: 0, green: 0, blue: 0 });
            }
        }

        self.row_position -= 1;
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}
