# Nano-9

Nano-9 is a Bevy library heavily inspired by Pico-8.

# Goals

The goals for Nano-9 are to

- offer a Pico-8-like API and semantics in both Rust and Lua,
- support the P8 cartridge format (potentially PNG),
- support different color palettes,
- support different color palette sizes,
- support different screen sizes,
- support different sprite sizes,
- support audio files,
- support audio files,
- provide a library first, and an application second,

# Anti-Goals

- Do not provide 100% compatibility with Pico-8.
- Do not support `peek()` or `poke()` in their entirety.
- Do not support same performance characteristics.
  
  Let me provide an example where Nano-9 and Pico-8 performance will differ. In
  Pico-8 if one doesn't clear the screen `cls()` and continues to draw sprites
  `spr()` each `_draw()`, the performance curve will be flat. However, in Nano-9
  a `spr()` creates a Bevy `Sprite` and if one doesn't clear them frequently,
  they will accumulate and degrade performance.
  
  Why not reify the last render to an image to preserve Pico-8's performance? One could do this certainly but my aim is to support Bevy's native elements as much as possible. I'd prefer for `spr()` to be a thin layer to Bevy's `Sprite`, `print()` to be `Text`, `map()` to be `bevy_ecs_tilemap`. In this way the comfortable world of Pico-8 can help introduce one to the Bevy world.

# Considerations

There are a number of questions that remain.

- Support Pico-8's Lua dialect?
  - If so, by what means? Text translation at load time, or compiler patch?
- Allow one to opt-in to retained entities.
 
  One of the principle differences between Pico-8 and Bevy is that Pico-8 has an
  what's called an immediate-mode rendering system. If you want to render a
  character say, you render its sprite
