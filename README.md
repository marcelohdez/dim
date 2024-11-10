# dim

Native Wayland screen dimming tool

## Usage

> [!NOTE]
> A Wayland compositor supporting the [single pixel buffer protocol] is
> required e.g. Sway 1.8+, river, Hyprland.

After [installing], you may run `dim` before you would run your locker, when
you want the screen to dim for a period, e.g. in your [swayidle] config/command:

```bash
timeout 270 'dim && swaylock'
```

Would make it so that at 270 seconds, `dim` is run waiting for user input
for the default of `30` seconds, then if no input is detected the next
command will proceed, in this case [swaylock] will lock your screen.

`dim` should only finish **successfully** when no input is detected for the
duration. If `dim` finishes successfully before this duration, please [submit
an issue].

The alpha and duration of `dim` may be configured with either a config file
located at `~/.config/dim/config.toml`, or through arguments at call-time, for
all options and their defaults please see:

```bash
dim --help
```

## Installing

dim packages are titled as `dim-screen` to avoid naming conflicts.

### Fedora (COPR)

dim is available in Fedora as a [COPR]:

```bash
sudo dnf copr enable marcelohdez/dim
sudo dnf install dim-screen
```

### Arch (AUR)

For Arch, dim is available in the [AUR] (Thanks to [ge-garcia] for
maintaining!). You may use your preferred [AUR helper] like so:

```bash
paru -Syu dim-screen
```

### Others

> [!IMPORTANT]
>
> - Ensure you have [Rust] installed.
> - The system libraries `libxkbcommon` and `libwayland` are required.

dim is available on crates.io:

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

[AUR]: https://aur.archlinux.org/packages/dim-screen
[ge-garcia]: https://github.com/ge-garcia/
[AUR helper]: https://wiki.archlinux.org/title/AUR_helpers
[COPR]: https://copr.fedorainfracloud.org/coprs/marcelohdez/dim
[installing]: #installing
[swayidle]: https://github.com/swaywm/swayidle
[swaylock]: https://github.com/swaywm/swaylock
[submit an issue]: https://github.com/marcelohdez/dim/issues
[Rust]: https://www.rust-lang.org/
[single pixel buffer protocol]: https://wayland.app/protocols/single-pixel-buffer-v1
[LICENSE]: LICENSE
