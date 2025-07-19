pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
-- a riff on this lib
-- https://github.com/automattf/vector.lua/blob/master/vector.lua
vector = {
  __add = function(a,b)
    return vec(a.x + b.x, a.y + b.y)
  end,

  __sub = function(a,b)
    return vec(a.x - b.x, a.y - b.y)
  end,

  __mul = function(a,b)
    -- assert(type(b) ~= 'number')
    if type(a) == 'number' then
      return vec(a * b.x, a * b.y)
    elseif type(b) == 'number' then
      return vec(a.x * b, a.y * b)
    else
      return vec(a.x * b.x, a.y * b.y)
    end
  end,

  __div = function(a,b)
    assert(type(b) == 'number' and type(a) ~= 'number')
    return vec(a.x / b, a.y / b)
  end,

  -- negate the vector
  __unm=function(self)
    return vec(-self.x, -self.y)
  end,

  --get the length of the vector
  length=function(self)
    return sqrt(self.x^2+self.y^2)
  end,

  --get the normal of the vector
  normalize=function(self)
    local l = self:length()
    self.x /= l
    self.y /= l
  end,
  
  cross = function(a, b)
    return a.x * b.y - b.x * a.y
  end,
  
  map = function(a, f, b)
    if (b) return vec(f(a.x, b.x), f(a.y, b.y))
    return vec(f(a.x), f(a.y))
  end,

  dot = function(a, b)

    return a.x * b.x + a.y * b.y
  end,

  reflect = function(self, mirror)
    return 2 * self:dot(mirror) * mirror - self
  end,
}
vector.__index = vector

function vec(x,y)
  local v = {x = x, y = y or x}
  setmetatable(v, vector)
  return v
end
