use crate::game::Tetrimino;
use alloc::vec::Vec;
use tinybmp::Bmp;

pub enum Sprite {
    Tetromino(Tetrimino),
    Wall,
    Empty,
    Border,
}

// Beautiful.
// I should write a macro for this
// Yes.
macro_rules! init_sprites {
    ($($file:literal),*) => {
        lazy_static! {
            static ref SPRITES: [Bmp<'static>; 11] = [$(Bmp::from_slice(include_bytes!(concat!("../assets/", $file))).unwrap()),*];
        }
    }
}
// less lines.
// allows me to add more sprites later easily.
// but ugly code.
// lets see if this works...

// RNG is very cryptographically secure.
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
    "empty.bmp",
    "border.bmp"
);
// i am thinking very hard
// very bad code
pub fn get_sprite(sprite: Sprite) -> &'static Bmp<'static> {
    &SPRITES[match sprite {
        Sprite::Tetromino(tetromino) => match tetromino {
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
        Sprite::Border => 10,
    }]
}
