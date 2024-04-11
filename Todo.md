# Things remaining

- [x] single-tap tilde `~`, grave`~`, and circumflex `^` on se-layouts
- [x] Massive cleanup, proper lints
- [x] Split into lib/bin to make tests work properly on x86_64
- [x] Clippy pedantic
- [x] Fixup oled on both sides
- [ ] Send multiple changes on the same report (mods and keypresses can go into the same, multiple keypresses) 
as well, but that only decreases latency if the user manager to generate more than 1 event per ms.
- [x] Fix bug where sometimes key-downs aren't registered
- [ ] Layer fallthrough, should have thought about this earlier
