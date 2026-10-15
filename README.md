# Steam Puck Flasher

A firmware flasher for the **Valve Steam Controller 2 puck** — the USB receiver..

It runs a guided GUI by default and offers the same flow in a terminal when run with the
`--cli` flag. It can install Valve's own builds, or unsigned images you have built
yourself, as there doesn't seem to be any kind of signature on the firmware.

Furthermore, this can be used to fix devices that have unprovisioned serial numbers due to prior expirimentation. The UICR, requires being provisioned in a very specific way, or the Serial number doesn't enumerate, which breaks charging.

The serial number is on the back of the puck, and you can enter this manually if the device is unprovisioned, and cnarging will once again work, getting you once again properly back to stock

As a note, I have not tested submitting a random serial number, so it's unclear if this is also valid.

---

# Not affiliated with Valve

This is an independent, reverse-engineered tool. It is not made, endorsed, or
supported by Valve. Flashing unofficial firmware to your hardware is done at your
own risk.

## What it does

The flasher walks through four steps:

- 1. **Serial** — you supply the puck's serial number manually or it will also be detected and the tool offers the serial it detects for you to confirm or correct.
- 2. **Catalogue** — pick a build. Valve's published builds are listed by channel
   (`stable` / `beta`) alongside any of your own images found in `./firmware/`.
- 3. **Confirm** — the chosen image is fetched and verified *before* you are asked,
   so you approve a real file rather than a promise from a remote index. These are taken from the [ibex-Firmware](https://github.com/OpenSteamController/Ibex-Firmware) catalogue.
- 4. **Flash** — the puck is put into bootloader mode, provisioned, written, and
   reset.

### Safety properties worth knowing

- **Every download is verified twice before it can be flashed.** The CRC32 covers
  the payload only, and the SHA256 covers the whole file including
  the 32-byte header. The two ranges differ, and getting that wrong makes good
  downloads look corrupt, so both are checked against the catalogue. A mismatch
  aborts and explains which hash disagreed.
- **A custom firmware is only checked for self-consistency**, because there is no
  upstream hash to compare against.
- **The provisioning block is read back and compared** byte-for-byte after it is
  written, and the flash is abandoned if it does not match.
- **Policy is provisioned before the first boot**: the application
  reads its hardware id and serial out of the UICR behind a magic-number gate, and
  a unit that boots without it comes up with a blank identity, and breaks charging.
- **The bootloader is not written.** Flashing replaces the application only, so a
  bad image leaves a device that can still be reflashed.
- **Downloads go in a temporary directory** as `*.part` and are renamed into
  place, so an interrupted transfer never leaves a truncated file that looks
  complete.

## Requirements

- The puck connected over USB.
- Rust 1.85 or newer.

> **Platform status.** This is developed and tested on Linux. The source is
> deliberately portable, nothing directly Linux specific, and is
> intended to build on Windows and macOS, but I haven't tested if these work.

### Linux permissions

Both the HID interface (used to request bootloader mode) and the CDC serial port
(used to talk to the bootloader) need to be accessible to your user. A udev rule
is the usual fix:

```udev
# /etc/udev/rules.d/70-steam-puck.rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="28de", MODE="0660", GROUP="plugdev"
SUBSYSTEM=="tty",    ATTRS{idVendor}=="28de", MODE="0660", GROUP="plugdev"
```

Then reload the rules and make sure you are in the `plugdev` group:

```sh
sudo udevadm control --reload-rules && sudo udevadm trigger
sudo usermod -aG plugdev "$USER"   # log out and back in for this to take effect
```

Running the tool with `sudo` also works, but the udev rule is better, it does not
leave root-owned files in your home directory.

## Building

```sh
git clone <this repo>
cd steam-puck-flasher
cargo build --release
```

The binary should be at `target/release/steam-puck-flasher`.

## Running

```sh
steam-puck-flasher              # GUI
steam-puck-flasher --cli        # Similar to GUI, but in a terminal
steam-puck-flasher --help
steam-puck-flasher --version
```

### Using custom firmware

Put any `.fw` container files in a `firmware/` folder **next to the executable**:

```
target/release/steam-puck-flasher
target/release/firmware/my-build.fw
```

They appear in the catalogue tagged `(user)`. The folder is only ever scanned,
never written to.

A `.fw` file is a 32-byte header followed by a flat image:

| offset | size | field |
|---|---|---|
| `0x00` | 4 | image magic (`0x2E795631` for the puck) |
| `0x04` | 4 | payload size |
| `0x08` | 4 | CRC32 of the payload |
| `0x0C` | 20 | reserved |
| `0x20` | — | payload |

### Debugging the puck

Set `STEAM_PUCK_FLASHER_TRACE=1` to log every bootloader frame to stderr. Talking
to the bootloader is the one part of this tool that cannot be exercised
off-device, and this makes the protocol inspectable without a debug build.

```sh
STEAM_PUCK_FLASHER_TRACE=1 steam-puck-flasher --cli
```
