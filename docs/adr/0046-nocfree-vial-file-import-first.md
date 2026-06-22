# NocFree Vial file import first

The first NocFree/Vial importer reads `.vil` JSON file exports rather than connecting to a live device. It produces an Import Candidate with Source Provenance, preserves raw UID, protocol versions, row layout arrays, macros, tap dance, combos, key overrides, settings, and other supported sections, and derives only a Fallback Layout when full per-key coordinate geometry is unavailable.
