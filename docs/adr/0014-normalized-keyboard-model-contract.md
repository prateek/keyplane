# Normalized Keyboard Model contract

The backend will expose a normalized Keyboard Model snapshot plus a runtime event stream to the frontend, rather than exposing source-specific protocol data directly. Capability flags and Source Provenance will travel with the model so the UI can show source health and confidence without coupling the overlay renderer to KeyPeek, Kanata, sentinel keys, polling, or other backend-specific integrations.
