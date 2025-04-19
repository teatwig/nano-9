# Nano-9

Nano-9 is Bevy in [Pico-8](https://www.lexaloffle.com/pico-8.php) clothing.

> [!WARNING]
> Nano-9 is currently in the early stages of development and is subject to
> breaking changes and not ready for public consumption. 

## Goals

The goals for Nano-9 are to

- offer a Pico-8 API and semantics in both Rust and Lua,
- support the P8 cartridge format (maybe PNG),
- provide a gateway from the Pico-8 world to the Bevy world,
- support different color palettes,
- support different color palette sizes,
- support different screen sizes,
- support different sprite sizes,
- support audio files,
- support different fonts,
- provide a library first, and an application second,
- support tilemap editors like [Tiled](http://www.mapeditor.org),
- and support unlimited code size.

## Anti-Goals

- Do not provide 100% compatibility with Pico-8. See [compatibility](compat.md) document.
- Do not provide a comprehensive game development console experience.
- Do not support `peek()` or `poke()` in their entirety.
- Do not write P8 or PNG cartridges.
- Do not use fixed-point numbers.
- Do not support same performance characteristics.
  
  Let me provide an example where Nano-9 and Pico-8 performance differ. In
  Pico-8 if one doesn't clear the screen `cls()` and continues to draw sprites
  `spr()` each `_draw()`, the performance curve will be flat. However, in Nano-9
  a `spr()` creates a Bevy `Sprite` and if one doesn't clear them frequently,
  they will accumulate and degrade performance.
  
  Why not reify the last render to an image to preserve Pico-8's performance?
  One could do this certainly but my aim is to support Bevy's native elements as
  much as possible. I'd prefer for `spr()` to be a thin layer to Bevy's
  `Sprite`, `print()` to create a `Text` component, `map()` to create a
  `bevy_ecs_tilemap::TilemapBundle`. In this way the comfortable world of Pico-8
  can help introduce one to the Bevy world, and it can also provide affordances
  not possible in Pico-8. For instance one could query on-screen entities for
  collision information.

## Current Design Considerations

There are a number of questions that remain unanswered.

### Support Pico-8's Lua dialect?
I would like to but by what means? Text translation at load time? A compiler
patch? Currently Nano-9 only supports Lua.

There are tools that help one convert Pico-8's dialect into conventional Lua:

- [pico8-to-lua](https://github.com/benwiley4000/pico8-to-lua)
- [Depicofier](https://github.com/Enichan/Depicofier)

But I haven't seen one that captures every part of Pico-8's dialect.

### Allow one to opt-in to retained entities?
One of the principle differences between Pico-8 and Bevy is that Pico-8 has an
what's called an immediate rendering system. If one wants to render a character,
one renders its sprite every frame. Bevy in contrast uses a retained rendering
system. One spawns a `Sprite` and that persists and is rendered every
frame until it is despawned.

One can imagine though that perhaps Nano-9's `spr()` function could be used like
so:

``` lua
function _init()
  a = spr(n):retain()
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
substitute Pico-8's reach; I want to extend it and introduce users to Bevy. A
case I imagine is this: someone creates a Pico-8 game, but they yearn to do
something that is not possible with Pico-8 itself. Like what? There are so many
things:

- Maybe use a full screen shader that creates a CRT effect, 
- maybe embed an arcade game in their actual game that is in fact a
Pico-8 game,
- maybe adjust the proportions of the screen so that it's a
landscape,
- maybe use a color palette that best expresses their aesthetic,
- maybe they need more buttons or access to a thumbstick,
- or maybe port their game to a console.

### Why Rust?

Nano-9 is built on Bevy, a game engine written in Rust. 

Here is Pico-8 code that draws a line from top-left of the screen to
the bottom-right.

``` lua
x = 0
function _update()
    pset(x, x)
    x = x + 1
end
```
Here is what the Rust version of the same thing looks like.

``` rust
fn update(mut pico8: Pico8, mut x: Local<u32>) -> Result<(), Pico8Error> {
    pico8.pset(*x, *x)?;
    *x += 1;
    Ok(())
}
```

### Can I use this to port my game to a console?

Hopefully, yes. Whatever consoles Bevy supports, Nano-9 should support too.
(However, I am not certain that bevy_mod_scripting supports WASM builds with
Lua.)

Some game developers have the technical wherewithal to rebuild their game in
another engine like the celebrated story of the
[Celeste](https://www.thatguyglen.com/article/MSKOQr_YS-U) by [Maddy
Thorson](https://www.maddymakesgames.com) and [Noel Berry](https://noelberry.ca)
which originated as a Pico-8 game before it was recreated in C# and XNA and
released to great success.

My hope is that Nano-9 offers a less-steep path for Pico-8 developers who, for
whatever reason, want more control over their game.

### Why is there no sprite, map, sfx, or music editor?

Because this is a library not a wholly integrated game development console like
Pico-8. Those things are all provided by different tools. Use whatever you like.
If you like the ones provided by Pico-8, use it! Here are some tools I like:

- I like [Aseprite](https://www.aseprite.org) for sprite editing.
- I like [Doom Emacs](https://github.com/doomemacs/doomemacs?tab=readme-ov-file) for text editing.
- I like [Bfxr](https://www.bfxr.net) for sound effects.
- I like [Tiled](http://www.mapeditor.org) for map editing.

### Why does `print()` create a tree of entities?

The Pico-8 TTF font this project uses has its height one pixel higher than
necessary; it's 7 pixels high. However, Pico-8 renders its text with 6 pixels of
verical spacing between lines. If one renders multi-line text naively with
Bevy's `Text::new(multi_line_string)` component, it will not look like Pico-8;
it will be off by 1 pixel each line, which for a small 128x128 display is a lot!
I tried to muck with the font in [FontForge](https://fontforge.org/en-US/) but
it was beyond my ability. So instead of using one `Text` component, Nano-9
creates a `Text` for each line under a root entity. It would be great to fix the
font for our use.

### Isn't this against Pico-8's purposefully constrained design philosophy?

Yes.

### Why buy Pico-8?

Because Pico-8 is great! It's limited for creativity's sake yet totally
comprehensive. Nano-9 doesn't aim to replicate the cohesive experience it
offers, which is utterly charming especially if you ever used an Apple IIe or
Commodore 64. I introduced my daughter to it a few years ago and she and I have
had a blast with it.

If you can't afford Pico-8, you can still play with it and learn it using the
[educational edition](https://www.pico-8-edu.com).

### Why not support `peek()` and `poke()`?

Pico-8 has easter-egg like features where if one tickles the right part of
memory, one _can_ access the keyboard keys or the mouse position, which are not
explicitly available via the API. While I think that is fun for a fantasy
console, I am not intent on replicating all that behavior. Instead I would
suggest that people extend the Lua API to provide access to whatever new
facilities they need.

If someone were to make a handful of useful `peek()` or `poke()` cases work, I
would consider such a pull request.

## Compatibility

| nano9 | bevy |
|-------|------|
| 0.1.0 | 0.15 |

## License

This crate is licensed under the MIT License or the Apache License 2.0.

## Acknowledgments

Many thanks to [Joseph "Zep" White](https://mastodon.social/@zep) the founder of
[Lexaloffle Games](https://www.lexaloffle.com) and creator of
[Pico-8](https://www.lexaloffle.com/pico-8.php).

Many thanks to the tireless work of [Maksymilian
Mozolewski](https://github.com/makspll) for
[bevy_mod_scripting](https://github.com/makspll/bevy_mod_scripting) without
which this project would not have been made.
