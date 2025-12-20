-- Original code[1] Copyright (c) 2023 PikuseruConsole[1]
-- Modified code Copyright (c) 2025 Shane Celis[2]
-- Licensed under the MIT License[3]
--
-- [1]: https://github.com/PikuseruConsole/pikuseru/
-- [2]: @shanecelis@mastodon.gamedev.place
-- [3]: https://opensource.org/licenses/MIT

local NUMBER_BITS <const> = 32
printh = print
debug_print = print
function on_script_loaded()
    if _init then
        --_init()
    end
end
-- Save the original
local _ipairs = ipairs

-- Overwrite ipairs to tolerate nil
function ipairs(t)
  if t == nil then
    -- Return an iterator that yields nothing:
    --   iter, state, init
    return function() return nil end, nil, nil
  end
  return _ipairs(t)
end

function min(a,b)
    if a == nil or b == nil then
            warn("min a or b are nil returning 0")
            return 0
    end
    if a < b then
        return a
    end
    return b
end
function max(a,b)
    if a == nil or b == nil then
            warn("max a or b are nil returning 0")
            return 0
    end
    if a > b then
        return a
    end
    return b
end
function mid(x, y, z)
    x = x or 0
    y = y or 0
    z = z or 0
    return x > y and x or y > z and z or y
end
function __pico_angle(a)
  -- FIXME: why does this work?
  return (((a - math.pi) / (math.pi*2)) + 0.25) % 1.0
end
flr = math.floor
ceil = math.ceil
cos = function(x) return math.cos((x or 0)*(math.pi*2)) end
sin = function(x) return math.sin(-(x or 0)*(math.pi*2)) end
function atan2(y, x)
    return __pico_angle(math.atan(y, x))
end
sqrt = math.sqrt
abs = math.abs
sgn = function(x)
    if x < 0 then
        return -1
    else
        return 1
    end
end
band = function(x, y)
  x = math.floor(x)
  y = math.floor(y)
  return x & y
end
bor = function(x, y)
  x = math.floor(x)
  y = math.floor(y)
  return x | y
end
bxor = function(x, y)
  x = math.floor(x)
  y = math.floor(y)
  return x ~ y
end
bnot = function(x)
  x = math.floor(x)
  return ~x
end

function add(a,v)
    if a == nil then
        warn("add to nil")
        return
    end
    table.insert(a,v)
end
function del(a,dv)
    if a == nil then
        warn("del from nil")
        return
    end
    for i,v in ipairs(a) do
        if v==dv  then
            return table.remove(a,i)
        end
    end
end

function deli(a,i)
    if a == nil then
        warn("del from nil")
        return
    end
    return table.remove(a,i)
end
function foreach(a,f)
    if not a then
        warn("foreach got a nil value")
        return
    end
    for i,v in ipairs(a) do
        f(v)
    end
end
function count(a,v)
    if v == nil then
        return #a
    else
        local count = 0
        for i,v in ipairs(a) do
            if v==dv then
                count = count + 1
            end
        end
        return count
    end
end
function all(a)
    local i = 0
    local n = #a
    return function()
        i = i + 1
        if i <= n  then
            return a[i]
        end
    end
end
-- string.sub does not respect UTF-8 boundaries.
-- sub = string.sub

function tonum(data)
    if type(data) == "number" then
        return data
    end

    if string.sub(data, 0, 2) == "0b" then
        local a, b=string.match(data,"(.*)%.(.*)$")
        if a == nil and b == nil then
            a = tonumber(string.sub(data, 3, #data), 2)
        return a
        end
        if a ~= nil and b ~= nil then
            a = tonumber(string.sub(a, 3, #a), 2)
            a = a + 0.5
            return a
        end
    end

    return tonumber(data, 10)
end


tostr = tostring
t = time
unpack = table.unpack

function split(inputstr, sep, convert_numbers)
    if sep == nil then
        sep = ","
    end
    local t={}
    for str in string.gmatch(inputstr, "([^"..sep.."]+)") do
            table.insert(t, str)
    end
    return t
end

function ord(str, index, count)
    if index == nil then
        index = 1
    end
    if count then
        local result = {}
        for i=0,count do
            table.insert(result, str:byte(index + i))
        end
        return unpack(table)

    else
        return str:byte(index)
    end
end

function menuitem(index, label, callback)
    debug_print("MENUITEM NOT IMPLEMENTED", nfadems, channelmask)
end

-- function assert(test, msg)
--     if ~test then
--         world.error(msg)
--     end
-- end

function stop(msg,x,y,color)
    debug_print("STOP NOT IMPLEMENTED")
end

function camera(x,y)
    local v = _camera(x,y)
    return v[1], v[2]
end

function xor(a, b)
    return (a or b) and not (a and b)
end

cocreate = coroutine.create
coresume = coroutine.resume
costatus = coroutine.status
yield = coroutine.yield

function _eval(str, output)
    local chunk, err = load("return " .. str)
    if not chunk then
        -- Try it without returning anything.
        chunk, err = load(str)
        if not chunk then
            return nil, err
        end
    end
    local v = chunk()
    if output and world.message then
        world.message(tostr(v))
    end
    return v
end

function cursor(x, y, c)
    local l = _cursor(x, y, c)
    return l[1], l[2], l[3]
end

-- function run(breadcrumb)
-- end
