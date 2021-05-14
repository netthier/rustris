#![no_std]
#![no_main]
#![feature(abi_efiapi)]
extern crate alloc;

#[macro_use]
extern crate lazy_static;

mod framebuffer;
mod game;
mod sprites;
mod ui;

use log::info;
use uefi::prelude::*;
use uefi_services::init;

#[entry]
fn efi_main(_image: Handle, sys_table: SystemTable<Boot>) -> Status {
    init(&sys_table).unwrap().unwrap();

    info!("Hello, world!");

    let mut game = game::Rustris::new();
    info!("Starting game...");
    game.start();
}
