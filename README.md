# Nano-9

Nano-9 is a fantasy console library heavily inspired by Pico-8.

## Goals

The goals for Nano-9 are to

- offer a Pico-8 API and semantics in both Rust and Lua,
- support the P8 cartridge format (maybe PNG),
- support different color palettes,
- support different color palette sizes,
- support different screen sizes,
- support different sprite sizes,
- support audio files,
- support different fonts,
- provide a library first, and an application second,
- and support unlimited code size.

## Anti-Goals

- Do not provide 100% compatibility with Pico-8.
- Do not support `peek()` or `poke()` in their entirety.
- Do not support same performance characteristics.
  
  Let me provide an example where Nano-9 and Pico-8 performance differ. In
  Pico-8 if one doesn't clear the screen `cls()` and continues to draw sprites
  `spr()` each `_draw()`, the performance curve will be flat. However, in Nano-9
  a `spr()` creates a Bevy `Sprite` and if one doesn't clear them frequently,
  they will accumulate and degrade performance.
  
  Why not reify the last render to an image to preserve Pico-8's performance?
  One could do this certainly but my aim is to support Bevy's native elements as
  much as possible. I'd prefer for `spr()` to be a thin layer to Bevy's
  `Sprite`, `print()` to be `Text`, `map()` to be `bevy_ecs_tilemap`. In this
  way the comfortable world of Pico-8 can help introduce one to the Bevy world.

## Considerations

There are a number of questions that remain unanswered.

### Support Pico-8's Lua dialect?
I would like to but by what means? Text translation at load time? A compiler
patch?

### Allow one to opt-in to retained entities?
One of the principle differences between Pico-8 and Bevy is that Pico-8 has an
what's called an immediate rendering system. If one wants to render a character,
one renders its sprite every frame. Bevy in contrast uses a retained rendering
system. One spawns a `Sprite` and that persists and is rendered every
frame until it is despawned.

One can imagine though that perhaps Nano-9's sprite function could be used like
so:

``` lua
function _init()
  a = spr(n).retain()
end

function _update()
  if btn(0) then
    a.x = a.x + 1
  end
  -- ...
end
```

One could opt-in to retained functionality. Is this a good idea? Retained mode
is more complicated to maintain but it is more performant. Happy to hear
feedback on this.

## FAQ

### Why a library?

Why not just offer an application like Pico-8? Because I don't mean to
substitute Pico-8's reach; I mean to lengthen it. One case I imagine is where
one creates a game using Pico-8, but they yearn to do something that is not
possible with Pico-8 itself. Like what? There are so many reasons: 

- Maybe use a full screen shader that creates a CRT effect. 
- Maybe they want to embed an arcade game in their actual game that is in fact a
Pico-8 game.
- Maybe they want to adjust the proportions of the screen so that it's a
landscape.
- Maybe they have a color palette that best expresses their aesthetic.
- Maybe they want to port their game to a console.

### Why does `print()` create a tree of entities?

The Pico-8 TTF font this project uses has its height one pixel higher than
necessary; it's 7 pixels high. However, Pico-8 renders its text with 6 pixels of
verical spacing between lines. If one renders multi-line text naively with
Bevy's `Text::new(multi_line_string)` component, it will not look like Pico-8;
it will be off by 1 pixel each line, which for a small 128x128 display is a lot!
I tried to muck with the font in FontForge but it was beyond my ability. So
instead of using one `Text` component, Nano-9 creates a `Text` for each line
under a root entity. It would be great to fix the font for our use.

### Isn't this against Pico-8's purposefully constrained design?

Yes.
