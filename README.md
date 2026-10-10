# The Incredible Machine · Rust

A multiplatform Rust/Bevy port of the 1992 DOS game *The Incredible Machine*.

The simulation is a 1:1 reimplementation of the original engine, verified against DOS captures for all 87 puzzles.

You need your own copy of the original game. Its art, sound and levels are loaded from it at runtime.

## Run

**[Play the web port in your browser](https://luthicc.github.io/the-incredible-machine-rust/)**

Or run it locally:

```sh
git clone https://github.com/luthicc/the-incredible-machine-rust.git
cd the-incredible-machine-rust
cargo run --release
```

Drop a ZIP of your original game into the window. Prebuilt Windows, macOS, Linux (AppImage) and browser builds are on the [Releases](../../releases) page.

## Quality of life

You can toggle each of these under **MENU**. None of them changes the simulation.

| Option | Default | What it does |
| --- | --- | --- |
| Resolution | — | Resizes the window to any 4:3 size |
| Pixel scaling | On | Scales by whole pixels for sharp edges |
| Stretch to fill window | Off | Fills the window instead of letterboxing |
| Smooth motion | On | Interpolates movement between simulation ticks |
| Puzzle hints / solutions | On | Reveals one part at a time or runs a full solution; save your own named solutions |
| Belt / rope connection hints | On | Shows valid connection points |
| Preview plugs while placing | On | Shows where an electrical plug will go |
| Z to flip a part | On | Flips the part you're holding |
| Slower picker buttons | On | Slows the item-picker scroll buttons |
| Mouse wheel in item picker | On | Scrolls the parts bin with the mouse wheel |
| Double click to copy | On | Double-click a placed part to pick up a copy |
| Shift to keep placing | On | Hold Shift to place several of the same part |
| Unlock all levels | Off | Opens every puzzle; your progress is unchanged |

On touch screens, drag a part and lift to drop it, tap empty space to put down a belt or rope, pinch to zoom the playfield and drag empty space to pan, and tap ← to go back. Ctrl + scroll wheel also zooms. The view zooms back out when the machine starts.

## Reference binary

The reverse engineering was done in Ghidra on this `TIM.EXE` (LZEXE-packed, 111,951 bytes):

```
SHA-256  1d8702c45106f4abd5af468c72c38d0588fc9e02a8928df988faaf57d5eb2303
```
