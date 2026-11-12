# Map, save and digest contracts

## Map v1

`schemas/map.schema.json` defines the shape; content/demo/map.json is a complete 12 x 8 example. Terrain symbols must be
defined exactly once in the legend. There are exactly height rows, each width ASCII symbols. Dimensions are 1..256.
Origin is top-left; x increases right and y down. Objects have unique stable ASCII IDs and in-bounds coordinates on
passable terrain. Multiple co-located services are represented by one location object, not competing occupancy records.

Terrain, objects and runtime object states remain separate. This file is immutable starting content; cleared encounters
and depleted stock are runtime state. Object kinds are town, encounter or objective in this first schema. Add new typed
variants when needed; reject unsupported variants instead of ignoring them.

## bounty-cjson-v1

This is a restricted project encoding, not an implementation of the complete JSON Canonicalization Scheme. Input is a
validated tree containing null, bool, integer, string, array and string-keyed object only. No floats, NaN, duplicate
object keys or unpaired Unicode surrogates. Integers are in [-2^63, 2^64-1] and use ordinary base-10 digits without a
plus, exponent or leading zeros; parsed negative zero is 0.

Object keys must be ASCII and are emitted in increasing byte order. Strings are UTF-8, with no Unicode normalization.
Escape double-quote and backslash; encode U+0000..001F as lowercase six-byte backslash-u00xx sequences. Do not escape
slash or printable non-ASCII characters. Booleans and null use lowercase JSON literals. Separate members/elements by
comma, key/value by colon, and add no whitespace, BOM or trailing newline. Arrays preserve their defined semantic order.

Before encoding a state, unordered entity collections are arrays sorted by stable ID. Army slot order, activation
queues, command tapes and event sequences are meaningful: never sort those arrays. Include all authoritative RNG and
policy state. Exclude frame interpolation, sound state, wall-clock timing and UI focus. A dedicated gameplay digest may
omit user-facing labels only if the omission is explicit in its projection schema. Do not silently change digest scope.

The digest is lowercase hex SHA-256 over precisely those UTF-8 bytes. File transport hashes, such as SHA256SUMS.txt,
instead cover exact file bytes; do not mix the two. Golden examples in `prototype/canonical_vectors.json` are checked by
the Rust tests of `bounty-core`, including control characters and non-ASCII values. (The specification pack shipped a
Python reference encoder that produced them; it is not kept in the repository.)

## Save envelope v1

Use `{save_schema_version:1, canonical_format:"bounty-cjson-v1", payload:..., sha256:...}`. The actual bytes are valid
JSON, including quoted keys; this line is shorthand. `sha256` covers canonical `payload` only. The outer file may be
pretty-printed. Payload includes ruleset/version/content identity, complete campaign state, optional complete battle
state, simulation RNG state and persistent AI state. Define the concrete Rust structs with the implemented game state;
this envelope is not a license to drop unknown fields. `schemas/save-envelope.schema.json` specifies the outer format;
the runtime additionally validates all payload state invariants.

Write the new file to the destination directory, validate it, then replace the current save using the platform's
supported safe replacement path. Keep one previous valid file. Report errors and preserve original bytes; do not
advertise a successful save after a failed write. Use isolated paths for tests. Power-loss durability must be tested on
claimed targets rather than assumed from a rename alone.

MVP reads the current schema and rejects unsupported ones. Do not implement or promise arbitrary migrations before a
supported old format exists. Mid-battle saves include the current actor, queues, flags and RNG, not an animation frame.
