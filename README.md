# Ocypode

I like Rust and I like SIM racing. I'm merging the two into this little project to read iRacing telemetry, display it live, and provide helpful alerts to improve driving style in real time. [Ocypodes are the fastest crabs](https://en.wikipedia.org/wiki/Ocypode).

## Status

The live stats are working, displaying basic telemetry and alerts. The offline analysis portion is not completed yet.

## Development
To keep the source code clean, we have a pre-commit git hook that runs the standard `fmt` and `clippy` checks. Before contributing code, run these commands in the repo root:

```sh
$ cargo install rustfmt
$ rustup component add clippy
$  git config --local core.hooksPath .githooks/
```