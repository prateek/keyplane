# First live validation target

Do not make core development depend on a specific physical keyboard. The first live-state path uses a fake Protocol Backend for CI and development, then validates against whichever KeyPeek-supported QMK/Vial/ZMK device is identified or flashed first. NocFree starts as an import-preview target, not a live-state target, unless compatible firmware support is added later.
