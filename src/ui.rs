use crate::framebuffer::Framebuffer;
use crate::game::{get_rotation, Matrix, Tetrimino};
use crate::sprites::{get_sprite, Sprite};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use embedded_graphics::drawable::Drawable;
use embedded_graphics::image::Image;
use embedded_graphics::{
    egrectangle, egtext, fonts::Font24x32, pixelcolor::Rgb888, prelude::*, primitive_style,
    text_style,
};

pub struct Ui<'a> {
    _score: u32,
    buffer: Framebuffer<'a>,
}

impl Ui<'_> {
    pub fn init() -> Self {
        let mut buffer = Framebuffer::new(800, 600);
        let t = egtext!(
            text = "Welcome to Rustris!",
            top_left = (184, 286),
            style = text_style!(font = Font24x32, text_color = Rgb888::WHITE)
        );
        t.draw(&mut buffer).unwrap();
        buffer.draw_buffer();
        //sleep(Duration::from_secs(2));
        buffer.clear(Rgb888::BLACK).unwrap();
        for y in 0..38 {
            for x in 0..50 {
                let image = Image::new(get_sprite(Sprite::Wall), Point::new(x * 16, y * 16));
                image.draw(&mut buffer).unwrap();
            }
        }

        let m = egrectangle!(
            top_left = (320, 160),
            bottom_right = (480, 480),
            style = primitive_style!(fill_color = Rgb888::BLACK)
        );

        m.draw(&mut buffer).unwrap();
        Self { _score: 0, buffer }
    }

    pub fn draw_matrix(&mut self, matrix: &Matrix) {
        let size = self.buffer.size();
        for (row, minos) in matrix.iter().rev().skip(20).take(20).enumerate() {
            for (col, mino) in minos.iter().skip(4).take(10).enumerate() {
                let sprite = if let Some(mino) = mino {
                    get_sprite(Sprite::Tetrimino(*mino))
                } else {
                    get_sprite(Sprite::Empty)
                };
                let image = Image::new(
                    sprite,
                    Point::new(
                        (size.width as usize / 2 - 5 * 16 + col * 16) as i32,
                        10 * 16 + (row as i32) * 16,
                    ),
                );
                image.draw(&mut self.buffer).unwrap();
            }
        }
    }
    pub fn draw_queue(&mut self, queue: &VecDeque<Vec<Tetrimino>>) {
        let q = egrectangle!(
            top_left = (496, 160),
            bottom_right = (576, 400),
            style = primitive_style!(fill_color = Rgb888::BLACK)
        );
        q.draw(&mut self.buffer).unwrap();
        let mut queue = queue.clone();
        for i in 0..5 {
            let next = if let Some(tetrimino) = queue[0].pop() {
                tetrimino
            } else {
                queue[1].pop().unwrap()
            };

            self.draw_piece(next, (504, 168 + i * 48));
        }
    }

    pub fn draw_hold(&mut self, content: &Option<Tetrimino>) {
        let h = egrectangle!(
            top_left = (224, 160),
            bottom_right = (304, 208),
            style = primitive_style!(fill_color = Rgb888::BLACK)
        );
        h.draw(&mut self.buffer).unwrap();
        if let Some(tetrimino) = content {
            self.draw_piece(*tetrimino, (230, 168));
        }
    }

    pub fn refresh(&mut self) {
        self.buffer.draw_buffer();
    }

    pub fn draw_piece(&mut self, tetrimino: Tetrimino, pos: (usize, usize)) {
        let block = (get_rotation(tetrimino) & 0xFFFF000000000000) >> 48;
        let sprite = get_sprite(Sprite::Tetrimino(tetrimino));
        for y in 0..4 {
            let row = (block & (0xF000 >> (y * 4))) >> (12 - y * 4);
            for x in 0..4 {
                if (row >> (3 - x)) % 2 == 1 {
                    let image = Image::new(
                        sprite,
                        Point::new(pos.0 as i32 + x * 16, pos.1 as i32 + y * 16),
                    );
                    image.draw(&mut self.buffer).unwrap();
                }
            }
        }
    }
}
