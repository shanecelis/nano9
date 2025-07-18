# TODO
## Pico-8
- [x] fix z-fighting, use frameCount + some increment
- [x] fix sub char splitting
- [x] Add pause or other state to stop Lua evaluation
      Can't use inspector while it's churning.
- [x] Add front matter to .lua and .p8lua files 
- [ ] Fix tiled import for lilly's house inside

## Nano-9
- [ ] Make Gfx Handles work with Maps.
- [ ] Remove the other .p8 loader?
- [x] cls() should be a trigger
- [x] Revert change to readonly sprite sheets
- [x] place gfx_material() on impl Pico8 directly.
- [ ] Remove dbg!s()
- [ ] Make sprites loadable from .p8 files.
- [ ] Fix no flags present for platformer.
      
- [x] Use Gfx for background pset() colors.
      Don't overwrite all colors unless it's marked dirty.
- [x] Use a 1x1 image for total background.
- [ ] Make pico-8 dialect work in .lua files.
- [x] try not to clone palettes (introduced during Pico8Asset refactor)
      Extract palettes to GfxHandler?
- [x] Add the ScriptComponent once
- [x] Load .p8 and .p8.png as a Pico8Asset in addition to Cart.
- [x] Remove error after reload
- [x] Make generic wrt palette bit-depth (at compile-time)
- [ ] Make generic wrt palette bit-depth at runtime
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
