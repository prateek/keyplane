# Semantic actions for visualization, not execution

The app will understand key actions enough to visualize and explain them, but not enough to reimplement firmware behavior. Each key keeps its Raw Action while deriving a normalized Semantic Action such as key, modifier, layer-momentary, layer-toggle, layer-tap, tap-hold, macro, transparent, none, mouse, or unknown; that semantic layer drives Display Legends, layer hints, and State Confidence warnings while raw source data remains preserved for exactness and future importer improvements.
