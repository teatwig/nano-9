# Nano-9

Nano-9 is Bevy in [Pico-8](https://www.lexaloffle.com/pico-8.php) clothing.

> [!WARNING]
> Nano-9 is currently in the early stages of development and is subject to
> breaking changes and not ready for public consumption. 
<p align="center">
  <img src="https://github.com/user-attachments/assets/307ff6e1-8682-4b99-979a-fcef7d3ae341"/>
</p>

## Goals

The goals for Nano-9 are to

- offer a Pico-8 API and semantics in both Rust and Lua,
- support the P8 and PNG cartridge format,
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
- Do not use fixed-point numbers in general.
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
  can help introduce one to the world of Bevy, and it can also provide affordances
  not possible in Pico-8. For instance one could query on-screen entities for
  collision information.

## Install

### As a repository

Recommended to easily exercise the examples.

``` sh
git clone https://github.com/shanecelis/nano-9.git
```

### As a library

Recommended if you are writing your game in Rust and not Lua.

``` sh
cargo add nano9@0.1.0-alpha.1
```

### As a command line tool n9

Recommended if you are writing your game in Lua and not Rust.
``` sh
cargo install nano9@0.1.0-alpha.1
```

## API Extensions

There are many extensions to the Pico-8 API usually in the form of extra
optional arguments at the end. For instance, Pico-8 has this signature for its
`print` function:

```lua
print(str, [x,] [y,] [col])
```

Nano-9's is the same with two additional arguments: font size and a font index
to select which font from the "Nano-9.toml" config file to use.

```lua
print(str, [x,] [y,] [col,] [font_size,] [font_index])
```

The rest of the extensions are indicated in italics in the
[compatibility](compat.md) document.

### Opt-in to retained entities 
One of the principle differences between Pico-8 and Bevy is that Pico-8 uses an
immediate rendering system. If one wants to render a character, one renders its
sprite every frame by calling `spr()`. Bevy in contrast uses a retained
rendering system. One spawns a `Sprite` and that persists and is rendered every
frame until it is despawned.

Nano-9 extends Pico-8's API for `spr()` by returning an `N9Entity`. This has a
handful of methods: `retain([z_position])`, `name([name])`, `pos(x, y, z)`,
`vis([visible])`, and `despawn()`.

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

## Examples
Many examples are written in both Lua and Rust to demonstrate how one can do
what they like with either language. The Lua examples can be run with `cargo run
FILE.lua`.

### hello-world
This one-liner prints "Hello".

To run the Lua version:
``` sh
cargo run examples/hello-world.lua
```
To run the Rust version:
``` sh
cargo run --example hello-world
```
### line
This example draws a line from the top-left to the bottom-right, one pixel per
frame.

``` sh
cargo run examples/line.lua
```
OR
``` sh
cargo run --example line; # Rust
```
### show-palette
This example draws the color palette as columns on the screen. You can choose
between the two templates: pico8 and gameboy. This example does not have a Lua
counterpart yet.

``` sh
cargo run --example show-palette pico8
```
OR
``` sh
cargo run --example show-palette gameboy
```
### sprite
<p align="center">
  <img src="https://github.com/user-attachments/assets/307ff6e1-8682-4b99-979a-fcef7d3ae341"/>
</p>

This example animates a sprite from a sprite sheet. This one is particularly
instructive because it shows the various ways of configuring and organizing a
Nano-9 game.

#### sprite/Nano9.toml

This is the conventional configuration where script, configuration, and assets
all go into one directory.

``` sh
cargo run example/sprite/Nano9.toml
```
OR
``` sh
cargo run example/sprite
```

#### sprite.p8lua

This one-file solution includes the following "front matter" to specify its
configuration: 

``` lua
--[===[
template = "pico8"
[[image]]
path = "BirdSprite.png"
sprite_size = [16, 16]
]===]
```

The front matter is interpreted by Lua as a comment, but the header and footer
must be exactly as they are in order for Nano-9 to process it as TOML
configuration data. The assets must come from an "assets" directory.

``` sh
cargo run examples/sprite.p8lua
```

#### sprite.rs
This example shows how one can configure Nano-9 within their Rust code. The
assets must come from an "assets" directory.

``` sh
cargo run --example sprite
```

#### sprite/Cargo.toml
There is one final form that a Nano-9 game may take and that's as its own cargo
project. There isn't an example for this one included in the repository. Until
there is the best example is a game jam that my nine-year-old daughter and I
made called [frog-oclock](https://github.com/rylandvonhunter/frog-oclock). That
project uses Lua and Rust. It is mainly a Lua project, but in the Rust code you
can see where an old TV post processing effect is commented out.

### n9 binary
The `cargo run` command is executing the `n9` binary, which can be installed.

``` sh
cargo install --path .
```

Once installed you can run an example like so:

``` sh
n9 examples/line.lua
```

However, the "sprite" example will probably produce an error saying it could not
find an asset in your "$HOME/.cargo/bin/assets" directory. This is due to Bevy's
standard behavior of looking for an "assets" directory where the executable is
stored. We can override that behavior by setting the NANO9_ASSET_DIR environment
variable.

``` sh
cd nano-9
NANO9_ASSET_DIR=assets n9 examples/sprite.p8lua
```

## Cargo Features

Nano-9 has a number of cargo features to tailor it to your use case. For
instance you can use it without "scripting", which means it will have no Lua
runtime at all.

### "scripting" (enabled by default)
Enables Lua scripting.

### "negate-y" (enabled by default)
Pico-8's positive y-axis points down the screen. Bevy's positive y-axis points
up by convention. This feature ensures that conversion happens. If it's
disabled, there will be no conversion, so it would be like using Pico-8 but with
y = 0 being the top of the screen and y = -127 being the bottom of the screen.

### "fixed" (enabled by default)
Pico-8's numbers are all 32-bit fixed-point numbers. Nano-9 using `f32`
generally. However, for a number of bit-twiddling functions like `shl()`,
`shr()`, `lshr()`, `rotr()`, and `rotl()` that difference may be noticeable. The
"fixed" feature converts `f32` to a fixed-point, does the operation then
converts it back to `f32`. If it's disabled, those bit operations are simply not
available (but perhaps they should be in the future).

### "pico8-to-lua" (enabled by default)
This enables conversion of Pico-8's dialect to regular Lua code.

### "web_asset" (disabled by default)
This enables one to place "http[s]://" URLs that will be resolved in the
Nano9.toml configuration file.

### "minibuffer" (disabled by default)
This enables [bevy_minibuffer](https://github.com/shanecelis/bevy_minibuffer)
for the "n9" CLI tool. It only has a few key bindings:

| ACT               | KEY BINDING |
|-------------------|-------------|
| toggle_pause      | Space N P   |
| lua_eval          | Space N E   |

### "inspector" (disabled by default)
This enables
[bevy_minibuffer_inspector](https://github.com/shanecelis/bevy_minibuffer_inspector) which allows one initiate [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui) from Minibuffer.

## FAQ

### Why a library?

Why not just offer an application like Pico-8? Because I don't mean to
substitute Pico-8's reach; I want to extend it on the one hand and introduce
users to Bevy on the other. A case I imagine is this: someone creates a Pico-8
game, but they yearn to do something that is not possible with Pico-8 itself.
Like what? There are so many things:

- Maybe use a full screen shader that creates a CRT effect, 
- maybe embed an arcade game in their actual game that is in fact a Pico-8 game,
- maybe adjust the proportions of the screen so that it's a landscape,
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
    x += 1
end
```

The above code works in Nano-9 in a .p8lua file. Below is the Lua code.
Note that Lua does not have the `+=` operator of Pico-8.

``` lua
x = 0
function _update()
    pset(x, x)
    x = x + 1
end
```

Here is the Rust version.
``` rust
use bevy::prelude::*;
use nano9::prelude::*;

fn update(mut pico8: Pico8, mut x: Local<u32>) {
    let _ = pico8.pset(UVec2::new(*x, *x), None);
    *x += 1;
}
```

### Does Nano-9 support Pico-8's Lua dialect?

Yes. Carts in .p8 or .png format can use Pico-8's dialect and its "#include"
syntax.

Files with the .lua extension will be interpreted as vanilla Lua files. Files
with the .p8lua extension will be translated to Lua from the Pico-8 dialect
using [pico8-to-lua](https://github.com/benwiley4000/pico8-to-lua). To see the
translated files, set the environment variable NANO9_LUA_CODE to a filename, and
Nano-9 will write the translated code and log it.

Note: pico8-to-lua is not fool proof. It relies on regular expression
substitution which do not faithfully parse the Pico-8 dialect for all valid
expressions. However, it allows many Pico-8 carts work without any changes. It
would be preferable to adopt a solution that actually parsed the Pico-8 dialect
like [Depicofier](https://github.com/Enichan/Depicofier) but for Rust.

### Can I use this to port my game to a console?

Hopefully, yes. Whatever consoles Bevy supports, Nano-9 should support too.
(However, bevy_mod_scripting currently [does not support
WASM](https://github.com/makspll/bevy_mod_scripting/issues/166) builds with
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

### What parts of `peek()`, `poke()`, and `stat()` are supported?

Nearly none currently. 

Pico-8 provides a memory-mapped interface for its more
esoteric features. For instance one _can_ access the keyboard keys or the mouse
position, which are not explicitly available via the API.

| Start   | End     | Purpose                                             |
|---------|---------|-----------------------------------------------------|
| 0x0     | 0x0fff  | Sprite sheet (0-127)*                               |
| 0x1000  | 0x1fff  | Sprite sheet (128-255)* / Map (rows 32-63) (shared) |
| 0x2000  | 0x2fff  | Map (rows 0-31)                                     |
| 0x3000  | 0x30ff  | Sprite flags                                        |
| 0x3100  | 0x31ff  | Music                                               |
| 0x3200  | 0x42ff  | Sound effects                                       |
| 0x4300  | 0x55ff  | General use (or work RAM)                           |
| 0x5600  | 0x5dff  | General use / custom font (0.2.2+)                  |
| 0x5e00  | 0x5eff  | Persistent cart data (64 numbers = 256 bytes)       |
| 0x5f00  | 0x5f3f  | Draw state                                          |
| 0x5f40  | 0x5f7f  | Hardware state                                      |
| 0x5f80  | 0x5fff  | GPIO pins (128 bytes)                               |
| 0x6000  | 0x7fff  | Screen data (8k)*                                   |
| 0x8000  | 0xffff  | General use / extended map (0.2.4+)                 |

Nano-9 does not in general support this memory-mapped interface. The interface
forces strong assumptions about how many sprites, maps, sound effects, and
music. Because Nano-9 breaks many of these assumptions, there is an impediment
to realizing them in a cogent way. For instance Nano-9 supports Pico-8 sound
effects but it also supports Ogg sound files. It's not clear how to support one
and possibly declaim the other.

Nano-9's Rust API for `peek()`, `poke()`, `stat()` return an error when
unsupported memory addresses or stat flags are given, which are currently most
of them.

#### What is supported?

Reading keyboard keys and mouse position and buttons are partially supported.

#### What is likely to be supported in the future?

The general use and persistent card data are likely to be supported in the future.

The more popular the memory-mapped feature is, the more likely it'll be supported.

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

Many thanks to the whole [Bevy team](https://bevyengine.org/community/people/)
for creating an exciting new open source game engine that has been a joy to work
with.

Many thanks to [ma9ici4n](https://itch.io/profile/ma9ici4n) for their
[16xBird](https://ma9ici4n.itch.io/pixel-art-bird-16x16) which is used in the
sprite example.
