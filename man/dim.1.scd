DIM(1)

# NAME

dim - A Wayland screen dimmer.

# USAGE

dim [OPTIONS]

# DESCRIPTION

dim requires a Wayland compositor implementing the wlr-layer-shell protocol.

Upon running, dim will create a black overlay with the given alpha (see
OPTIONS) and wait for the given duration (see OPTIONS). If any mouse, touch or
keyboard input is detected, an error will be thrown. Otherwise, when the
duration is reached without input, dim will quit successfully, allowing you to
chain commands. For example, if using `swayidle` you may set:

```
timeout 270 'dim && swaylock'
```

Making it so at 270 seconds, or 5 minutes, dim is run, and if no input is
detected after its timeout, swaylock will be run, locking the screen.

# OPTIONS

\-d, --duration <DURATION>
	Set the duration in seconds. 0 Will be considered as infinite. The default is
	30.

\-a, --alpha <ALPHA>
	Set the *alpha* value of the overlay, 0.0 being transparent and 1.0 being
	solid black. When solid, cursor will be hidden. Default is 0.5.

\-p, --passthrough
	Make dim ignore input, passing it to the surfaces behind it, making dim act as
	a way to lower your brightness artificially. You probably want to set the
	duration option above to 0 for this to last indefinitely.

	When scripting this, you could summon dim with passthrough enabled and then
	kill it with `pkill dim`

\--gen-completions <PATH>
	Generates completions for all supported shells at the given path.

\-c, --config <PATH>
	Read config file at given PATH instead of default location (see CONFIGURATION
	below.)

\-h, --help
	Print help information

\-V, --version
	Print the binary's version

# CONFIGURATION

dim can be configured through a TOML configuration file, which is by default
looked for at `$XDG_CONFIG_HOME/dim/config.toml`, or, if `$XDG_CONFIG_HOME` is
not set, it will instead default to searching at `~/.config/dim/config.toml`.

The options set here will be overriden by any arguments passed in. Valid
options are alpha, duration and passthrough as seen above, example config:

```
# i am a comment!
duration = 30
alpha = 0.5
passthrough = false
```
