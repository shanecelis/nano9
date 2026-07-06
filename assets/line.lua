-- BUG: It looks like _init isn't called. That's why this bare `x = 0` is required.
x = 0
function _init()
    cls()
    x = 0
end

function _update()
    pset(x, x)
    -- can't use pico8 dialect here.
    x = x + 1
end
