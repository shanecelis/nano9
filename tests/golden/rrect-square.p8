pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- golden: rrect / rrectfill on squares (w=h=2*r+1)
-- Odd size so diameter matches circ/circfill(r).
-- max_r for size 2*r+1 is r (floored min/2).
--
-- row0: rrectfill r=0..8  color 8
-- row1: circfill  r=0..8 at same centers  color 11
-- row2: rrect     r=0..8  color 12
-- spacing: size+2

function _draw()
  cls(0)
  local y0, y1, y2 = 4, 40, 76
  local x = 2
  for r=0,8 do
    local s = 2 * r + 1
    rrectfill(x, y0, s, s, r, 8)
    local cx = x + r
    local cy = y0 + r
    circfill(cx, y1 + r, r, 11)
    rrect(x, y2, s, s, r, 12)
    x += s + 2
  end
  if stat(6)=="headless" then
    extcmd("set_filename", "rrect-square")
    extcmd("screen", 1, 1)
    extcmd("shutdown")
  end
end
