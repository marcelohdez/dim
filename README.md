# dim

Native Wayland screen dimming tool

## Usage

After [installing], you may run `dim` before you would run your locker, when
you want the screen to dim for a period, e.g. in your [swayidle] config:

```bash
timeout 270 'dim && swaylock'
```

Would make it so that at 270 seconds, `dim` is run waiting for user input
for the default of `30` seconds, then if no input is detected the next
command will proceed, in this case [swaylock] will lock your screen.

`dim` should only finish **successfully** when no input is detected for the
duration given with the `--duration` command, which is 30 by default. If
`dim` finishes successfully before this duration, please [submit an issue].

The alpha of `dim` may be configured with the `--alpha` option. For more info,
please see:

```bash
dim --help
```

## Installing

Ensure you have [Rust] installed.

dim is available on crates.io as `dim-screen` to avoid naming conflicts but
the binary is still `dim`:

```bash
cargo install dim-screen
```

### Building Manually

Choose a directory for this repo, then clone and `cd` into it:

```bash
git clone https://github.com/marcelohdez/dim
cd dim
```

Lastly, `cargo` can build and install `dim` for you, placing the binary in
`$HOME/.cargo/bin/`:

```bash
cargo install --path .
```

Or, if you would like to place the binary in your `$PATH` yourself:

```bash
cargo build -r
```

And the resulting binary should be in `./target/release/dim`.

## License

`dim` is licensed under the GPLv3 license, a free and open source license. For
more information, please refer to the [LICENSE] file in the repository root.

[installing]: https://github.com/marcelohdez/dim/#installing
[swayidle]: https://github.com/swaywm/swayidle
[swaylock]: https://github.com/swaywm/swaylock
[submit an issue]: https://github.com/marcelohdez/dim/issues
[Rust]: https://www.rust-lang.org/
[LICENSE]: https://github.com/marcelohdez/dim/blob/master/LICENSE
