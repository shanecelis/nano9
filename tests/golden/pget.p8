pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- golden: pget
-- green squares when each read matches pico-8. a red square is a miss.
function _draw()
  cls(1)
  pset(3, 4, 8)
  -- a later draw starts another pixel canvas. the first stays visible,
  -- transparent everywhere except (3, 4).
  rectfill(0, 0, 0, 0, 7)
  pset(6, 6, 12)
  if pget(3, 4) == 8 then
    rectfill(8, 8, 15, 15, 11)
  else
    rectfill(8, 8, 15, 15, 8)
  end
  if pget(6, 6) == 12 then
    rectfill(20, 8, 27, 15, 11)
  else
    rectfill(20, 8, 27, 15, 8)
  end
  pset(1, 1, 0)
  if pget(1, 1) == 0 then
    rectfill(32, 8, 39, 15, 11)
  else
    rectfill(32, 8, 39, 15, 8)
  end
  if stat(6) == "headless" then
    extcmd("set_filename", "pget")
    extcmd("screen", 1, 1)
    extcmd("shutdown")
  end
end
