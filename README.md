# ugokuna

## What's this?

Command line utility to download _ugoira_ from pixiv

## Why is it named so?

_ugoira_ is a _moving illustration_, and this tool makes it _stay_ (on your drive)

_plz kill me_

## Dependencies

Have `ffmpeg` in your `PATH`.

As no binary releases are provided, rust toolchain is also required.

## Usage

```sh
$ cargo run --quiet -- --help
```

```
ugokuna 

USAGE:
    ugokuna.exe [OPTIONS] <OUT_PATH>

ARGS:
    <OUT_PATH>    

OPTIONS:
    -f, --format <FORMAT>              [default: gif] [possible values: webm, gif]
    -h, --help                         Print help information
    -i, --illust-ids <ILLUST_IDS>
    -p, --profile-ids <PROFILE_IDS>
```
