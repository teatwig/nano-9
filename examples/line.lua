function _init()
    cls()
    x = 0
end

function _update()
    pset(x, x)
    -- can't use pico8 dialect here.
    x = x + 1
end
