pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
-- actor.p8

actor = {
  sprite = nil, -- start sprite
  x = nil,
  y = nil,
  w = 8,
  h = 8,
  frame = 0,
  frames = 1,
}

-- make an actor
-- x,y means center of the actor
-- in map tiles
function actor:new(a, sprite, x, y)
  a = a or {}
  setmetatable(a, self)
  self.__index = self
  a.sprite = sprite or a.sprite
  a.x = x or a.x
  a.y = y or a.y
  return a
end

function actor:update()
end

function actor.draw(a)
  spr(a.sprite + (flr(a.frame) % a.frames) * a.w / 8, a.x, a.y, a.w / 8, a.h / 8)
end

function actor:in_bounds(x, y)
  return x >= self.x and x < (self.x + self.w) and y >= self.y and y < (self.y + self.h)
end

function actor.is_sprite(a, s)
  return s >= a.sprite and s < a.sprite + a.frames
end

if mouse then
  widget = actor:new {
    -- can_click
    update = function(self)
      local inside = self:in_bounds(mouse.x, mouse.y)
      if (self.on_hover) self:on_hover(inside)
      if inside then
        if (mouse:btnp() and self.on_down) self:on_down()
        if (mouse:btnp_up() and self.on_up) self:on_up()
        end
      end
  }
end

-- actor_with_particles = actor:new {
--   emitter = nil
-- }

-- function actor_with_particles:new(o, sprite, x, y)
--   o = actor.new(self, o, sprite, x, y)
--   if (o.emitter) o.emitter = o.emitter:clone()
--   return o
--   end

-- function actor_with_particles:update()
--   actor.update(self)
--   if self.emitter then
--     self.emitter.pos.x = self.x * 8 - 4
--     self.emitter.pos.y = self.y * 8 - 4
--     self.emitter.p_angle = atan2(-self.dx, -self.dy)
--     self.emitter:update(delta_time)
--   end
-- end
-- function actor_with_particles:draw()
--   actor.draw(self)
--   if (self.emitter) self.emitter:draw()
-- end
