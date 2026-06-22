# Active layer stack

Active layer state will be modeled as an ordered Layer Stack rather than a single current layer. Runtime State should carry active layer IDs, Layer Precedence, Activation Kind when known, and State Confidence; the overlay may highlight the top active layer, but Effective Actions resolve per Physical Key through the ordered stack so momentary layers, toggles, tap-holds, default layers, and transparent keys behave coherently.
