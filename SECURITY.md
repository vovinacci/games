# Security

These are single-player games, but they do a few things worth reporting problems with: they read and write save files,
may read game data from a folder the player picks, and will download and install their own updates, verifying a
signature first.

If you find a way to make a game run code it shouldn't, install an unsigned update, or damage files outside its data
folder, please report it privately via
[GitHub's private vulnerability reporting](https://github.com/vovinacci/games/security/advisories/new) rather than a
public issue. Only the latest release of each game is supported.
