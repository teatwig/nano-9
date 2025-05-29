function _init()
    cls()
    x = 0
end

function _update()
    pset(x, x)
    x = x + 1
end
