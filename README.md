# 💾 bd
Bulk Data Duplicator (dd)

- Slick CLI interface
- Burn ISOs to multiple drives in parallel
- Backup to multiple files at once

## Usage
Similar to `dd`

See `bd --help` for a full list of CLI arguments.

```bash
# Backup /dev/sda
bd -i=/dev/sda -o=./sda.img
```

```bash
# Backup to multiple places in parallel
bd -i=/dev/sda -o ./localbackup.img /nas/offsitebackup.img
```

```bash
# Burn ISO to USB
bd -i=./2019-09-26-raspbian-buster-lite.img -o=/dev/sdb
```

```bash
# Burn ISO to multiple USBs
bd -i=./2019-09-26-raspbian-buster-lite.img -o /dev/sdb /dev/sdc /dev/sdd
```

## Installation

```bash
git clone https://github.com/ajmwagar/bd
cd bd
cargo install --path=.
```
