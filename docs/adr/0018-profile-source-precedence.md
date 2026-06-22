# Profile source precedence

Profiles will resolve conflicting imported data through explicit per-field Source Precedence, with User Overrides always taking priority. For the MVP, KeyPeek or firmware-aware sources win for Runtime State, imported Vial/VIA/ZMK/KeyPeek data wins for Physical Layout and Logical Keymap, Kanata wins only for runtime layer state unless paired with companion profile data, and keyviz style JSON wins only for Visual Style fields; losing values remain available through Source Provenance but are not rendered by default.
