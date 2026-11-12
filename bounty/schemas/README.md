# Schemas

map.schema.json validates the authoring shape. Semantic checks additionally require row widths/heights, known symbols,
unique IDs and in-bounds/passable object locations. The loader in `bounty-core` performs those checks, with the demo map
as its test.

save-envelope.schema.json fixes the outer representation and required payload sections. It does not claim to enumerate a
complete not-yet-implemented game state. The engine must deserialize campaign/battle/RNG/policy sections into concrete
Rust types with all required fields and validate their invariants. Reject unknown schema versions and duplicate JSON
keys; do not infer Classic defaults from missing fields.
