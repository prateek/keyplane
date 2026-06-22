# Backend failures as runtime state

Permissions, connection failures, stale sources, and backend errors will be modeled as Runtime State and Backend Health rather than one-off errors. The overlay and app shell should be able to persistently show states such as missing accessibility permission, disconnected HID, unavailable Kanata TCP, or stale Best-Effort Preview data instead of relying on transient notifications or logs.
