# Rustris
Very bad (feature- and code-wise) Tetris clone, made to run as an UEFI executable.
Tries to follow Tetris guideline as much as possible (SRS, 7-bag, hold, etc.).

![](https://i.imgur.com/iJ2onc9.png)


## TODO
- [ ] Fix ghost pieces disappearing if piece is too close to them
- [x] ~~Fix lock down timer not starting after certain twists~~
- [x] ~~Fix memory leak (?) causing the game to crash after a few minutes~~ Watchdog timer :)
- [ ] Add 15 move rule to lock down (Extended Placement)
- [ ] Make "holding keys down" actually work, may require a keyboard driver
- [ ] Add scoring
- [ ] Increase gravity with time
- [ ] Add multiplayer

## Build instructions
- `rustup update`
- `rustup override set nightly`
- `cargo build --target x86_64-unknown-uefi -Z build-std=core,compiler_builtins,alloc -Z build-std-features=compiler-builtins-mem`
  (Or `cargo kbuild` if using custom config.toml)
  
- `cargo run --package disk_image -- target/x86_64-unknown-uefi/debug/rustris-efi.efi`

