# wl-distore

A program that manages your display configuration on wl-roots compositors
automatically in the background.

## Overview

On `wl-roots` compositors, you are able to configure your displays (e.g.,
setting the resolution of displays, moving displays around). Some tools like
[`wdisplays`](https://github.com/artizirk/wdisplays) help with configuring your
displays, but `wl-roots` compositors don't save this configuration by default.
The changes to your displays may only persist until the compositor restarts.

That's where `wl-distore` comes in. `wl-distore` listens to changes to your
display configuration and saves the configuration. When a monitor is plugged in
or out, the saved configuration is applied. This allows you to modify your
configuration using whatever tool you want (like `wdisplays`), and have that
configuration automatically persist!

## Getting Started

The first step is installation. This requires Rust, which can either be
installed through your package manager, or through https://rustup.rs. You can
then run:

```bash
cargo install wl-distore
```

After that, you can run it with:

```bash
wl-distore
```

Then configure your displays however you like! The next time `wl-distore`
detects those sets of monitors, that configuration will be applied (assuming
`wl-distore` is running).

### Launching `wl-distore` automatically

`wl-distore` needs to be running to save and apply configurations. Each
compositor will have a different way of achieving this. For example, using
[Sway](https://swaywm.org/), one way to achieve this is by adding the following
to your config:

```
exec $HOME/.cargo/bin/wl-distore
```

Alternatively, you can use a `systemd` user service. Sway documents this
workflow [here](https://github.com/swaywm/sway/wiki/Systemd-integration). An
example of a service file is:

```systemd
[Unit]
Description = "wl-distore"
PartOf=graphical-session.target

[Service]
Type=simple
Environment=RUST_LOG=info
ExecStart=%h/.cargo/bin/wl-distore

[Install]
WantedBy=sway-session.target
```

## Configuration

The default configuration file lives at `~/.config/wl-distore/config.toml`. Use
the `--config` flag to change this. The config file options include:

- `layouts`: The file path to where layouts are saved. Defaults to
  `~/.local/state/wl-distore/layouts.json`.
- `apply_command`: The shell command to run after a layout is applied.

## Alternatives

### [kanshi](https://sr.ht/~emersion/kanshi/)

`kanshi` is a similar tool, where you define your desired display configurations
in a config file and these configurations are applied whenever a match is found.
This means that you have a "canonical" source for your display configurations.

The disadvantage to this workflow is it's all manual! If you configure your
displays through a tool like `wdisplays`, your configurations won't
automatically persist and you must figure out how to match that configuration in
the config file.

In contrast, `wl-distore` automatically persists everything. The cost is less
control - any change to the display configuration is persisted. In addition, the
lack of a config file for layouts makes it difficult to specify explicit
layouts.

## Why is it called wl-distore?

Mostly because I'm bad at naming things. It's meant to be (W)ay(l)and (di)splay
(store).

## License

License under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Any contribution submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional
terms or conditions.