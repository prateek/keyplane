# EDN v1 profile codec

Use app-native EDN v1 with namespaced keywords for sections, fields, and enum-like values; strings for Stable Element IDs and user-authored identifiers; vectors for ordered data; and maps for keyed collections. Hide parsing, writing, and migrations behind a Rust Profile Codec. The first codec may use `edn-rs` plus serde, but deterministic save formatting and schema migration behavior belong to the app.
