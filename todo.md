# TODO
## Pico-8
- [x] fix z-fighting, use frameCount + some increment
- [x] fix sub char splitting
- [x] Add pause or other state to stop Lua evaluation
      Can't use inspector while it's churning.
- [ ] Add front matter to .n9 file which is .toml followed by .lua
- [x] Add sspr for character faces
- [ ] Fix tiled import for lilly's house inside
## Nano-9
- [ ] try not to clone palettes (introduced during Pico8Asset refactor)
- [x] Add the ScriptComponent once
- [x] Load .p8 and .p8.png as a Pico8Asset in addition to Cart.
- [x] Remove error after reload
- [ ] Make generic wrt palette bit-depth
- [x] Allow multiple palettes
- [ ] Check collisions example
- [ ] make sprite flags generic
- [x] add full screen key (alt-enter)
- [x] scale image with window
- [x] implement cls()
- [x] audio sfx
- [ ] audio music
- [x] audio control
- [x] implement tile map
- [x] show errors
- [x] make work with local paths

## Bugs
- [x] _draw() gets called before Pico8State is loaded.
