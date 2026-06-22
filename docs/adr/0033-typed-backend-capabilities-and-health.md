# Typed backend capabilities and health

Protocol Backends will report typed Capability Flags and typed Backend Health rather than ad hoc strings. Each backend declares whether it can discover devices, import geometry or keymaps, stream live Layer Stack changes, stream pressed keys, poll state, or only provide Best-Effort Preview; health states include ok, permission-missing, disconnected, stale, unsupported, parse-error, and protocol-error.
