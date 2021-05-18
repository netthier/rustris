use crate::game::Tetrimino;
use tinybmp::Bmp;

pub enum Sprite {
    Tetrimino(Tetrimino),
    Wall,
    Empty,
}

macro_rules! init_sprites {
    ($($file:literal),*) => {
        lazy_static! {
            static ref SPRITES: [Bmp<'static>; 10] = [$(Bmp::from_slice(include_bytes!(concat!("../assets/", $file))).unwrap()),*];
        }
    }
}

init_sprites!(
    "o-piece.bmp",
    "i-piece.bmp",
    "t-piece.bmp",
    "l-piece.bmp",
    "j-piece.bmp",
    "s-piece.bmp",
    "z-piece.bmp",
    "ghost.bmp",
    "wall.bmp",
    "empty.bmp"
);

pub fn get_sprite(sprite: Sprite) -> &'static Bmp<'static> {
    &SPRITES[match sprite {
        Sprite::Tetrimino(tetrimino) => match tetrimino {
            Tetrimino::O => 0,
            Tetrimino::I => 1,
            Tetrimino::T => 2,
            Tetrimino::L => 3,
            Tetrimino::J => 4,
            Tetrimino::S => 5,
            Tetrimino::Z => 6,
            Tetrimino::Ghost => 7,
        },
        Sprite::Wall => 8,
        Sprite::Empty => 9,
    }]
}
