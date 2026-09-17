pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- golden: non-square rrect / rrectfill
-- top:  40x20 fills  r=0,2,4,10(max),11(max+1)
-- mid:  20x40 fills  r=0,2,4,10,11
-- bot:  50x30 fill r=7; outlines 34x30 r=0 and 32x30 r=15(max)
-- also: 18x40 outline r=13 (max=9, so max+1 probe)

function _draw()
  cls(0)
  local rs = {0, 2, 4, 10, 11}
  local w, h, gap = 40, 20, 42
  for i, r in ipairs(rs) do
    local col = (i - 1) % 3
    local row = flr((i - 1) / 3)
    rrectfill(2 + col * gap, 2 + row * 22, w, h, r, 8)
  end

  w, h, gap = 20, 40, 22
  for i, r in ipairs(rs) do
    rrectfill(2 + (i - 1) * gap, 50, w, h, r, 11)
  end

  rrectfill(2, 96, 50, 30, 7, 12)
  rrect(56, 96, 34, 30, 0, 10)
  rrect(94, 96, 32, 30, 15, 10)
  rrect(108, 2, 18, 40, 13, 9)

  if stat(6)=="headless" then
    extcmd("set_filename", "rrect-rect")
    extcmd("screen", 1, 1)
    extcmd("shutdown")
  end
end
