# Games

Small games written in Rust, for the fun of making and playing them. One repository, one cargo workspace; every game is
built and released on its own.

| Game | What it is | State |
|---|---|---|
| [Bounty](bounty/) | A tribute to King's Bounty (1990): explore, recruit, fight, find the sceptre before time runs out | Just begun; nothing to play yet |

## Building

Rust is pinned in `mise.toml` ([mise](https://mise.jdx.dev) installs it). Then:

```sh
make          # lists the targets
make test
make lint
```

`make hooks-install` sets up the git hooks, which run the same checks as CI.

## Licence

MIT, see [LICENSE](LICENSE). The games are original code; none ships files from the games they pay tribute to.
