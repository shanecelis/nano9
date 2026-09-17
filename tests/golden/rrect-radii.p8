pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- golden: fixed odd square, r = 0..8, max, max+1
-- size 19x19 => min/2 = 9.5; probe r=0..8,9,10
-- row0 fills:  r=0..5
-- row1 fills:  r=6,7,8,9,10 + circfill(r=9) for compare
-- row2/3 outlines

function _draw()
  cls(0)
  local s = 19
  local gap = 21
  local rs0 = {0, 1, 2, 3, 4, 5}
  local rs1 = {6, 7, 8, 9, 10}

  for i, r in ipairs(rs0) do
    local x = 2 + (i - 1) * gap
    rrectfill(x, 2, s, s, r, 8)
    rrect(x, 66, s, s, r, 12)
  end
  for i, r in ipairs(rs1) do
    local x = 2 + (i - 1) * gap
    rrectfill(x, 34, s, s, r, 8)
    rrect(x, 98, s, s, r, 12)
  end
  -- circfill(r=9) beside last fill; center = box origin + 9
  circfill(2 + 5 * gap + 9, 34 + 9, 9, 11)

  if stat(6)=="headless" then
    extcmd("set_filename", "rrect-radii")
    extcmd("screen", 1, 1)
    extcmd("shutdown")
  end
end
