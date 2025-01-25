-- Original code[1] Copyright (c) 2023 PikuseruConsole[1]
-- Modified code Copyright (c) 2025 Shane Celis[2]
-- Licensed under the MIT License[3]
--
-- [1]: https://github.com/PikuseruConsole/pikuseru/
-- [2]: @shanecelis@mastodon.gamedev.place
-- [3]: https://opensource.org/licenses/MIT

printh = print
debug_print = print
function on_script_loaded()
    if _init then
        --_init()
    end
end

function btnp(b)
    return __btnp(b)
end

function btn(b)
    return __btn(b)
end

function cls(c)
    return __cls(c)
end

function pset(x, y, c)
    return __pset(x, y, c)
end

function rect(x0,y0,x1,y1,c)
    return __rect(x0,y0,x1,y1,c)
end

function rectfill(x0,y0,x1,y1,c)
    return __rectfill(x0,y0,x1,y1,c)
end

function spr(n,x,y,w,h,fx,fy)
    return __spr(n,x,y,w,h,fx,fy)
end

function map(cx,cy,sx,sy,cw,ch,l)
    return __map(cx or 0,cy or 0,sx or 0,sy or 0,cw or 128,ch or 64,l)
end

function print(s,x,y,c)
    return __print(s,x,y,c)
end

function sfx(n, c, o, l)
    return __sfx(n, c, o, l)
end

function fget(n, f)
    return __fget(n, f)
end

function fset(n, f, v)
    return __fset(n, f, v)
end

function sub(s, start, stop)
    return __sub(s, start, stop)
end

function camera(x, y)
    return __camera(x, y)
end

function line(x0,y0,x1,y1,c)
    return __line(x0,y0,x1,y1,c)
end

function circfill(x0,y0,r,c)
    return __circfill(x0,y0,r,c)
end

function circ(x0,y0,r,c)
    return __circ(x0,y0,r,c)
end

function rnd(v)
    return __rnd(v)
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
shl = function(x, y)
  x = math.floor(x)
  y = math.floor(y)
  return x << y
end
shr = function(x, y)
  x = math.floor(x)
  y = math.floor(y)
  return x >> y
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
function foreach(a,f)
    if not a then
        warn("foreach got a nil value")
        return
    end
    for i,v in ipairs(a) do
        f(v)
    end
end
function count(a)
    return #a
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

function stat(val)
    debug_print("STAT NOT IMPLEMENTED", val)
    return nil
end

function poke(addr, value)
    debug_print("POKE NOT IMPLEMENTED", addr, value)
end

-- empty commands
function music(n, fadems, channelmask)
    debug_print("MUSIC NOT IMPLEMENTED", nfadems, channelmask)
end

function menuitem(index, label, callback)
    debug_print("MENUITEM NOT IMPLEMENTED", nfadems, channelmask)
end

function assert(test, msg)
    if ~test then
        world.error(msg)
    end
end

function stop(msg,x,y,color)
    debug_print("STOP NOT IMPLEMENTED")
end

cocreate = coroutine.create
coresume = coroutine.resume
costatus = coroutine.status
yield = coroutine.yield

-- function run(breadcrumb)
-- end
