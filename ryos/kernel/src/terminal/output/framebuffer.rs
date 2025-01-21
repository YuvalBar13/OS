use bootloader_api::info::{FrameBuffer, PixelFormat};
use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{Rgb888, RgbColor},
};
use conquer_once::spin::OnceCell;
use spin::Mutex;
use noto_sans_mono_bitmap::{
    get_raster, get_raster_width, FontWeight, RasterHeight, RasterizedChar,
};

pub static DEFAULT_COLOR: Color = Color { red: 255, green: 255, blue: 255 };
pub static ERROR_COLOR: Color = Color { red: 255, green: 0, blue: 0 };

// Global writer instance using OnceCell
pub static WRITER: OnceCell<Mutex<Writer>> = OnceCell::uninit();

// Initialize the global writer
pub fn init_writer(framebuffer: FrameBuffer) {
    let writer = Writer::new(
        framebuffer,
        DEFAULT_COLOR.clone(),
        RasterHeight::Size32,
        FontWeight::Regular,
    );
    WRITER.init_once(|| Mutex::new(writer));
    WRITER.get().expect("Writer not initialized").lock().clear_screen_with_color(Color { red: 0, green: 0, blue: 0 });
}

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

pub fn set_pixel_in(framebuffer: &mut FrameBuffer, position: Position, color: Color) {
    let info = framebuffer.info();

    let byte_offset = {
        let line_offset = position.y * info.stride;
        let pixel_offset = line_offset + position.x;
        pixel_offset * info.bytes_per_pixel
    };

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

pub struct Writer {
    column_position: usize,
    row_position: usize,
    color_code: Color,
    buffer: FrameBuffer,  // Now owns the FrameBuffer
    font_height: RasterHeight,
    font_weight: FontWeight,
}

impl Writer {
    pub fn new(
        buffer: FrameBuffer,
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

    pub fn change_color(&mut self, color: Color)
    {
        self.color_code = color;
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
                    &mut self.buffer,
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
                    set_pixel_in(&mut self.buffer, pos, color);
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
        for y in 0..char_height {
            for x in 0..info.width {
                let pos = Position {
                    x,
                    y: info.height - char_height + y,
                };
                set_pixel_in(&mut self.buffer, pos, Color { red: 0, green: 0, blue: 0 });
            }
        }

        self.row_position -= 1;
    }

    pub fn backspace(&mut self) {
        if self.column_position > 0 {
            self.column_position -= 1;
            // Clear the character at the current position
            let char_width = self.char_width();
            let char_height = self.char_height();

            // Clear the character space
            for y in 0..char_height {
                for x in 0..char_width {
                    let pos = Position {
                        x: self.column_position * char_width + x,
                        y: self.row_position * char_height + y,
                    };
                    set_pixel_in(&mut self.buffer, pos, Color { red: 0, green: 0, blue: 0 });
                }
            }
        }
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

