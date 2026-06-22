# Sentinel keys as a protocol backend

Sentinel-key layer tracking will be modeled as its own Protocol Backend. It listens to Host Input Events and maps configured Sentinel Keys to layer changes through the active Profile, but it must advertise lower State Confidence than firmware-aware or authoritative remapper backends because startup state, dropped events, and out-of-band layer changes can make it wrong.
