# Per-key coordinate geometry

The canonical Physical Layout model will use per-key coordinate geometry, not row arrays. Each Physical Key will have a Stable Element ID, optional Matrix Position, x/y/w/h geometry, optional rotation, and Source Provenance; OverKeys-style row arrays may import as a Fallback Layout, but the normalized model must match KeyPeek, Vial, and ZMK-style real geometry so split, angled, and non-rectangular keyboards render correctly.
