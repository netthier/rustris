use crate::framebuffer::Framebuffer;
use crate::sprites::{get_sprite, Sprite};
use crate::ui::Ui;
use alloc::collections::vec_deque::VecDeque;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::BorrowMut;
use core::convert::TryFrom;
use core::ops::BitOr;
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use embedded_graphics::image::Image;
use log::info;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use uefi::data_types::chars::Char16;
use uefi::proto::console::text::{Input, Key, ScanCode};
use uefi::table::boot::{EventType, TimerTrigger, Tpl};
use uefi::Event;
use uefi_services::system_table;

pub type Matrix = [[Option<Tetrimino>; 18]; 44];
static FALL_SPEED: u64 = 1000;
static DROP_FLAG: AtomicBool = AtomicBool::new(false);
static LOCKDOWN_FLAG: AtomicBool = AtomicBool::new(false);

pub struct GameData {
    pub active_mino: (Tetrimino, usize, usize, u8), // type, x, y, rot
    pub queue: VecDeque<Vec<Tetrimino>>,
    pub hold: Option<Tetrimino>,
    pub hold_flag: bool,
    pub state: GameState,
    pub matrix: Matrix,
    pub rng: SmallRng,
}

impl GameData {
    pub fn get_piece(&mut self) -> Tetrimino {
        while self.queue.len() < 2 {
            self.push_bag();
        }

        if self.queue[0].is_empty() {
            self.queue.pop_front();
            self.push_bag();
        }
        self.queue[0].pop().unwrap()
    }

    fn push_bag(&mut self) {
        let mut bag = vec![
            Tetrimino::O,
            Tetrimino::I,
            Tetrimino::T,
            Tetrimino::L,
            Tetrimino::J,
            Tetrimino::S,
            Tetrimino::Z,
        ];
        bag.shuffle(&mut self.rng);
        self.queue.push_back(bag);
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum GameState {
    Spawn,
    Drop,
    ClearLines,
    Die,
}

pub struct Rustris<'a> {
    data: GameData,
    ui: Ui<'a>,
    keyboard: &'a mut Input,
    drop_event: Event,
    lockdown_event: Event,
    waiting_lockdown: bool,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Tetrimino {
    O,
    I,
    T,
    L,
    J,
    S,
    Z,
    Ghost,
}

impl Rustris<'_> {
    pub fn new() -> Self {
        let drop_event = unsafe {
            system_table()
                .as_ref()
                .boot_services()
                .create_event(
                    EventType::TIMER.bitor(EventType::NOTIFY_SIGNAL),
                    Tpl::NOTIFY,
                    Some(tick_piece),
                )
                .unwrap()
                .unwrap()
        };

        let lockdown_event = unsafe {
            system_table()
                .as_ref()
                .boot_services()
                .create_event(
                    EventType::TIMER.bitor(EventType::NOTIFY_SIGNAL),
                    Tpl::NOTIFY,
                    Some(tick_lockdown),
                )
                .unwrap()
                .unwrap()
        };

        let seed = unsafe {
            system_table()
                .as_ref()
                .runtime_services()
                .get_time()
                .unwrap()
                .unwrap()
                .second()
        };

        info!("Seed: {}", seed);

        let data = GameData {
            active_mino: (Tetrimino::O, 0, 0, 0),
            queue: VecDeque::new(),
            hold: None,
            hold_flag: false,
            state: GameState::Spawn,
            matrix: [[None; 18]; 44], // + 4 buffer blocks on each side
            rng: SmallRng::seed_from_u64(seed as u64),
        };

        let protocol = unsafe {
            system_table()
                .as_ref()
                .boot_services()
                .locate_protocol::<Input>()
                .unwrap()
                .unwrap()
        };

        let keyboard = unsafe { &mut *protocol.get() };
        let mut ui = Ui::init();
        ui.draw_hold(&None);
        Self {
            data,
            ui,
            keyboard,
            drop_event,
            lockdown_event,
            waiting_lockdown: false,
        }
    }

    pub fn start(&mut self) -> ! {
        loop {
            self.step();
        }
    }

    fn step(&mut self) {
        match self.data.state {
            GameState::Spawn => {
                let tetrimino = self.data.get_piece();
                self.ui.draw_queue(&self.data.queue);
                let res = tetrimino.spawn(&mut self.data);
                self.ui.refresh();
                if res {
                    self.data.state = GameState::Die;
                } else {
                    self.data.state = GameState::Drop;
                    DROP_FLAG.store(true, Ordering::Relaxed);
                }
            }
            GameState::Drop => {
                if DROP_FLAG.load(Ordering::Relaxed) {
                    self.move_piece_no_check((0, -1));
                    self.ui.draw_matrix(&self.data.matrix);
                    self.ui.refresh();
                }

                if LOCKDOWN_FLAG.load(Ordering::Relaxed) {
                    LOCKDOWN_FLAG.store(false, Ordering::Relaxed);
                    self.manage_lockdown_timer(true);
                    self.data.state = GameState::ClearLines;
                }

                if let Some(key) = self.keyboard.read_key().unwrap().unwrap() {
                    match key {
                        Key::Special(ScanCode::LEFT) => self.move_piece_no_check((-1, 0)),
                        Key::Special(ScanCode::RIGHT) => self.move_piece_no_check((1, 0)),
                        Key::Special(ScanCode::DOWN) => self.move_piece_no_check((0, -1)),
                        Key::Printable(e) => match e.into() {
                            ' ' => {
                                LOCKDOWN_FLAG.store(true, Ordering::Relaxed);
                                while !self.move_piece((0, -1)) {}
                            }
                            'e' => self.rotate_piece(1),
                            'q' => self.rotate_piece(3),
                            'f' => {
                                self.hold_piece();
                                self.ui.draw_hold(&self.data.hold);
                                self.ui.draw_queue(&self.data.queue);
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    self.ui.draw_matrix(&self.data.matrix);
                    self.ui.refresh();
                }
            }
            GameState::ClearLines => {
                'outer: loop {
                    let clone = self.data.matrix;
                    for (idx, row) in clone.iter().enumerate() {
                        if row.iter().skip(4).take(10).all(|e| e.is_some()) {
                            for y in idx..39 {
                                self.data.matrix[y] = self.data.matrix[y + 1];
                                if self.data.matrix[y + 1].iter().all(|e| e.is_none()) {
                                    break;
                                }
                            }
                            continue 'outer;
                        }
                    }
                    break 'outer;
                }
                self.ui.draw_matrix(&self.data.matrix);
                self.ui.refresh();
                self.data.state = GameState::Spawn;
            }
            GameState::Die => {
                panic!("u ded lol");
            }
        }
    }

    fn move_ghost(&mut self, delete: bool) {
        let ghost_mino = Tetrimino::Ghost;
        let (_, x, y, rot) = self.data.active_mino;
        let mut counter = 4;
        while !ghost_mino.place(&mut self.data, (x, y - counter), rot, false, true) {
            ghost_mino.place(&mut self.data, (x, y - counter + 1), rot, true, false);
            ghost_mino.place(&mut self.data, (x, y - counter), rot, delete, false);
            counter += 1;
        }
    }

    fn hold_piece(&mut self) {
        if !self.data.hold_flag {
            let (old_mino, x, y, rot) = self.data.active_mino;
            self.move_ghost(true);
            old_mino.place(&mut self.data, (x, y), rot, true, false);
            if let Some(tetrimino) = self.data.hold {
                tetrimino.spawn(&mut self.data);
            } else {
                self.data.get_piece().spawn(&mut self.data);
            }
            self.data.hold = Some(old_mino);
            self.data.hold_flag = true;
            DROP_FLAG.store(true, Ordering::Relaxed);
            self.ui.refresh();
        }
    }

    fn rotate_piece(&mut self, drot: u8) {
        let mut active_mino = self.data.active_mino;

        let (tetrimino, x, y, rot) = (
            active_mino.0,
            active_mino.1,
            active_mino.2,
            (active_mino.3 + drot) % 4,
        );
        self.move_ghost(true);
        if tetrimino.rotate(&mut self.data, (x, y), active_mino.3, rot) {
            self.manage_lockdown_timer(true);
        }
        self.move_ghost(false);
    }

    fn move_piece_no_check(&mut self, dpos: (isize, isize)) {
        self.move_piece(dpos);
    }

    fn move_piece(&mut self, dpos: (isize, isize)) -> bool {
        let mut active_mino = self.data.active_mino;

        let (tetrimino, x, y, rot) = (
            active_mino.0,
            (active_mino.1 as isize + dpos.0) as usize,
            (active_mino.2 as isize + dpos.1) as usize,
            active_mino.3,
        );

        tetrimino.place(
            &mut self.data,
            (active_mino.1, active_mino.2),
            active_mino.3,
            true,
            false,
        );

        let res = tetrimino.place(&mut self.data, (x, y), rot, false, true);

        if res {
            tetrimino.place(
                &mut self.data,
                (active_mino.1, active_mino.2),
                active_mino.3,
                false,
                false,
            );

            if dpos == (0, -1) {
                self.data.hold_flag = false;
                return true;
            }
        } else {
            if self.waiting_lockdown {
                self.manage_lockdown_timer(true);
            }
            self.move_ghost(true);
            self.data.active_mino = (tetrimino, x, y, rot);
            // Test if tetrimino has hit ground
            if tetrimino.place(&mut self.data, (x, y - 1), rot, false, true) {
                if !self.waiting_lockdown {
                    self.manage_lockdown_timer(false);
                }
            }
            tetrimino.place(&mut self.data, (x, y), rot, false, false);
            self.move_ghost(false);
        }
        if dpos == (0, -1) {
            DROP_FLAG.store(false, Ordering::Relaxed);
            self.start_drop_timer();
        }

        false
    }

    fn start_drop_timer(&self) {
        unsafe {
            system_table()
                .as_ref()
                .boot_services()
                .set_timer(self.drop_event, TimerTrigger::Relative(FALL_SPEED * 10000))
                .unwrap()
                .unwrap();
        }
    }

    fn manage_lockdown_timer(&mut self, cancel: bool) {
        self.waiting_lockdown = !cancel;
        unsafe {
            system_table()
                .as_ref()
                .boot_services()
                .set_timer(
                    self.lockdown_event,
                    if cancel {
                        TimerTrigger::Cancel
                    } else {
                        TimerTrigger::Relative(5000000)
                    },
                )
                .unwrap()
                .unwrap();
        }
    }
}

pub fn tick_piece(_: Event) {
    DROP_FLAG.store(true, Ordering::Relaxed);
}
pub fn tick_lockdown(_: Event) {
    LOCKDOWN_FLAG.store(true, Ordering::Relaxed);
}

impl Tetrimino {
    pub fn spawn(self, mut data: &mut GameData) -> bool {
        let pos = (7, 25);
        let res = self.place(&mut data, pos, 0, false, true);
        self.place(&mut data, pos, 0, false, false);
        data.active_mino = (self, pos.0, pos.1, 0);
        res
    }

    pub fn rotate(
        self,
        mut data: &mut GameData,
        pos: (usize, usize),
        old_rot: u8,
        new_rot: u8,
    ) -> bool {
        let mut tests: Vec<(isize, isize)> = vec![(0, 0)];

        if data.active_mino.0 != Tetrimino::I {
            for i in 0..4 {
                tests.push(match (old_rot, new_rot) {
                    (0, 1) | (2, 1) => match i {
                        0 => (-1, 0),
                        1 => (-1, 1),
                        2 => (0, -2),
                        3 => (-1, -2),
                        _ => unreachable!(),
                    },
                    (1, 0) | (1, 2) => match i {
                        0 => (1, 0),
                        1 => (1, -1),
                        2 => (0, 2),
                        3 => (1, 2),
                        _ => unreachable!(),
                    },
                    (2, 3) | (0, 3) => match i {
                        0 => (1, 0),
                        1 => (1, 1),
                        2 => (0, -2),
                        3 => (1, -2),
                        _ => unreachable!(),
                    },
                    (3, 2) | (3, 0) => match i {
                        0 => (-1, 0),
                        1 => (-1, -1),
                        2 => (0, 2),
                        3 => (-1, 2),
                        _ => unreachable!(),
                    },

                    _ => unreachable!(),
                });
            }
        } else {
            for i in 0..4 {
                tests.push(match (old_rot, new_rot) {
                    (0, 1) | (3, 2) => match i {
                        0 => (-2, 0),
                        1 => (1, 0),
                        2 => (-2, 1),
                        3 => (1, 2),
                        _ => unreachable!(),
                    },
                    (1, 0) | (2, 3) => match i {
                        0 => (2, 0),
                        1 => (-1, 0),
                        2 => (2, 1),
                        3 => (-1, -2),
                        _ => unreachable!(),
                    },
                    (1, 2) | (0, 3) => match i {
                        0 => (-1, 0),
                        1 => (2, 0),
                        2 => (-1, 2),
                        3 => (2, -1),
                        _ => unreachable!(),
                    },
                    (2, 1) | (3, 0) => match i {
                        0 => (1, 0),
                        1 => (-2, 0),
                        2 => (1, -2),
                        3 => (-2, 1),
                        _ => unreachable!(),
                    },

                    _ => unreachable!(),
                });
            }
        }

        self.place(&mut data, pos, old_rot, true, false);
        for (off_x, off_y) in tests.iter() {
            let (x, y) = (
                (pos.0 as isize + off_x) as usize,
                (pos.1 as isize + off_y) as usize,
            );
            if !self.place(&mut data, (x, y), new_rot, false, true) {
                self.place(&mut data, (x, y), new_rot, false, false);
                data.active_mino.1 = x;
                data.active_mino.2 = y;
                data.active_mino.3 = new_rot;
                return true;
            }
        }
        self.place(&mut data, pos, old_rot, false, false);
        false
    }

    fn matrix_draw(
        self,
        mut data: &mut GameData,
        pos: (usize, usize),
        test: bool,
        delete: bool,
    ) -> bool {
        let mino = data.matrix[pos.1][pos.0].borrow_mut();
        if test {
            !(4..14).contains(&pos.0)
                || !(4..44).contains(&pos.1)
                || (mino.is_some() && *mino != Some(Tetrimino::Ghost))
        } else {
            if self != Tetrimino::Ghost || !mino.is_some() || *mino == Some(Tetrimino::Ghost) {
                *mino = if delete { None } else { Some(self) };
            }
            false
        }
    }

    // pos is top-left corner of: https://tetris.fandom.com/wiki/SRS?file=SRS-pieces.png
    pub fn place(
        self,
        mut data: &mut GameData,
        pos: (usize, usize),
        rot: u8,
        delete: bool,
        test: bool,
    ) -> bool {
        let rot = rot % 4;

        let mino = if self != Tetrimino::Ghost {
            self
        } else {
            data.active_mino.0
        };

        let rot_data = get_rotation(mino);

        let block = ((rot_data & (0xFFFF000000000000 >> rot * 16)) >> (48 - rot * 16)) as u16;
        for y in 0..4 {
            let row = (block & (0xF000 >> y * 4)) >> (12 - y * 4);
            for x in 0..4 {
                if (row >> (3 - x)) % 2 == 1 {
                    if self.matrix_draw(&mut data, (pos.0 + x, pos.1 - y), test, delete) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/*
All 4 rotations of a piece encoded as a u64.
// TODO explain this lol
 */
pub const ROTATIONS: [u64; 7] = [
    0x6600660066006600, // O
    0x0F00222200F04444, // I
    0x4E0046400E404C40, // T
    0x2E0044600E80C440, // L
    0x8E0064400E2044C0, // J
    0x6C00462006C08C40, // S
    0xC60026400C604C80, // Z
];

pub fn get_rotation(tetrimino: Tetrimino) -> &'static u64 {
    &ROTATIONS[match tetrimino {
        Tetrimino::O => 0,
        Tetrimino::I => 1,
        Tetrimino::T => 2,
        Tetrimino::L => 3,
        Tetrimino::J => 4,
        Tetrimino::S => 5,
        Tetrimino::Z => 6,
        _ => unreachable!(),
    }]
}
