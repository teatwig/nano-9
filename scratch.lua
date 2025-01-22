-- text:print("Hello, World!")

world.info("Lua: The scratch.lua script just got loaded")
    -- print("hr")
c = {r = 0, g = 0, b = 1}
function _init()
world.info("Lua: The scratch.lua script just got INIT")
    -- print("hi")
    -- cls({r = 0, g = 0, b = 0})
    -- background.image:set_pixel(16,32, c)
    -- pixie = nano9.image:load("images/pixie.png")
    -- -- jar = pixie:spr(64,64)
    -- jar = pixie:sprite()
    -- jar.one_frame = false

    -- local Transform = world:get_type_by_name("Transform")
    -- local t = Transform.from_xyz(0,1, 2)
    -- local t = world:get_component(entity,Transform)
    -- jar = nil


    -- sprite = nil
    -- sprite.x = 64
    -- sprite.y = 64
    -- pixie:spr()
end

x = 0
function _update()
    -- background.name = "what"
    --background.Sprite.flip_x = x % 2 == 0
    -- jar2.Sprite.flip_x = x % 10 == 0


    x = x + 1
end

function _draw()
    cls(1)
    -- btn()
    pset(x + 10,x, 2)
    -- background.image:set_pixel(x, x, c)
    -- spr(1, x, x)
    -- jar = pixie:sprite()
    -- jar.one_frame = true
    print("a", 0, 0, 6)
    for i = 1,19 do
        -- print(tostr(i))
    end
    print("hiHI", 64, 0, 6)
    -- print("oh")
end
