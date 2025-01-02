-- text:print("Hello, World!")

c = {r = 0, g = 0, b = 1}
function _init()
    cls(c)
    background.image:set_pixel(16,32, c)
    pixie = image:load("images/pixie.png")
    -- jar = pixie:spr(64,64)
    local jar = pixie:sprite()
    jar2 = jar

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
    background.image:set_pixel(x, x, c)
    --background.Sprite.flip_x = x % 2 == 0
    -- jar2.Sprite.flip_x = x % 10 == 0


    x = x + 1
end
