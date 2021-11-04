# Chip-8 Emulator

CHIP-8 emulator in Rust. Not fully tested, but should work except for sound.

Build and run using [Cargo](https://doc.rust-lang.org/cargo/).

```
Usage: chip8 [OPTIONS]

Positional arguments:
  filename                    Path to the CHIP-8 ROM

Optional arguments:
  --bitshift-ignores-vy       Shift instruction uses VX without first setting VX to VY
  --jump-with-offset-uses-vx  Jump-with-offset adds VX instead of V0
  --add-to-index-ignores-overflow
                              Add to index instruction does not set VF on overflow
  --store-and-load-increment-index
                              Increment index register by X after store and load
  -h, --help                  Print help message
```

### Key mapping
| | | | |<--Chip-8/Keyboard-->| | | | |
|---|---|---|---|---|---|---|---|---|
|`1`|`2`|`3`|`C`||`1`|`2`|`3`|`4`|
|`4`|`5`|`6`|`D`||`Q`|`W`|`E`|`R`|
|`7`|`8`|`9`|`E`||`A`|`S`|`D`|`F`|
|`A`|`0`|`B`|`F`||`Z`|`X`|`C`|`V`|
