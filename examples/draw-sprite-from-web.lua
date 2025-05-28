===
template = "pico8"
[[image]]
path = "BirdSprite.png"
sprite_size = [16, 16]
===
function _init()
    color(6)
    print("hello")
    spr(0)
end

x = 0
function _draw()
    cls()

    spr(time() % 8 + 8, x)
    x = x + 1
end
