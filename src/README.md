# Source map

| Area | Where |
| --- | --- |
| Entry point, Bevy setup, input, frame timing | `main.rs`, `app/` |
| ZIP import and the import window | `importer/`, `launcher/` |
| Objects, save format, rope geometry, drawing | `machine/` |
| Simulation step and callback dispatch | `simulation/machine_simulation.rs`, `simulation/dispatch.rs` |
| Per-part behaviour | `simulation/parts/<part>.rs` |
| Gears and motors | `simulation/rotation.rs` |
| Ropes | `simulation/rope_contacts.rs`, `rope_dynamics/` |
| Collision and motion | `contact.rs`, `spatial/`, `outline.rs`, `forces.rs`, `motion/` |
| Editor | `editor/` |
| Puzzles, goals, progress | `puzzle/`, `goals/`, `progress/` |
| Menus and screens | `frontend/`, `menus/`, `game_screen/` |
| Solutions (87 bundled) | `solutions/`, `default_solutions/` |
| Resources, sound, saves, tables | `resources/`, `sound/`, `storage/`, `tables/` |
| Browser shell | `web/` |

## Keeping it faithful

Simulation order is defined in code, not by how files are laid out: `MachineSimulation::step`, callback dispatch, and the Bevy system registration. Do not change list sorting, insertion order or object indices. Saves keep the original record layout.

Definition IDs, state values and flag words remain raw native integers. Named flags only describe known bits. Unknown bits keep numeric names such as `ReservedBit14`. Comments include the original routine names and addresses so you can compare the code with the DOS binary.

## Tests

```sh
cargo fmt --check
cargo check --all-targets
cargo check --lib --bins --target wasm32-unknown-unknown
TIM_GAME_DATA=/path/to/game-folder cargo test --lib --bins
```

Tests sit next to the code as `tests.rs` or `*_tests.rs`. Small fixtures are in `test_data/`. Tests that need game resources read them from `TIM_GAME_DATA`.
