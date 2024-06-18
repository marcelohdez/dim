# Example configs

## config.toml

Example dim config.

## swayidle

Adapted from my own dotfiles, it does several things;

At 45 seconds idle, check if `swaylock` (or your locker) is running, if so,
suspend again (user has been idle for 45 seconds on lock screen):

```bash
timeout 45 'pgrep swaylock && systemctl suspend'
```

At 435 seconds (7 minute and 15 seconds) it runs dim, which if no input is
detected after the default of 30 seconds, it will exit successfully and run
the screen locker 15 seconds before the system is suspended at 480 seconds:

```bash
timeout 435 'dim && swaylock'
timeout 480 'systemctl suspend'
```

Finally, before the system sleeps (even if user suspends manually) pause all
media players and run screen locker:

```bash
before-sleep 'playerctl pause -i kdeconnect; swaylock'
```
