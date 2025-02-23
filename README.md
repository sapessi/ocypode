# Ocypode

I like Rust and I like sim racing. These two passions meet in this little project to read iRacing telemetry, display it live, and provide helpful alerts to improve driving skills in real-time. In case you are wondering, [Ocypodes are the fastest crabs](https://en.wikipedia.org/wiki/Ocypode).

## Why Ocypode
There are lots of telemetry overlays out there. However, I couldn't find one that **(1) gave you a real-time, intuitive view of your driving errors, and (2) didn't require some sort of paid subscription.**

### Real-time alerts 
Traditional telemetry tools require that you save telemetry data and then dive deep to find out what you did wrong and when. Analyzing telemetry data is time-consuming and requires a lot of expertise.

Ocypode analyzes telemetry data in real-time to show intuitive alerts for excessive braking force, wheelspin, missed shifts, etc. This allows you to improve your skills while you drive, without having to dig into the data or switch context.

### Free and open source
I want Ocypode to remain a free, open-source tool

## Status
The real-time view with basic telemetry and alerts is working. The offline analysis portion is lower priority for a first release. I have created [a project](https://github.com/users/sapessi/projects/1/views/1) to track the first official release.

## Development
To keep the source code clean, we have a pre-commit git hook that runs the standard `fmt` and `clippy` checks. Before contributing code, run these commands in the repo root:

```sh
$ cargo install rustfmt
$ rustup component add clippy
$  git config --local core.hooksPath .githooks/
```