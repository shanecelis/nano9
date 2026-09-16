pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- golden: sfx-loop
local t = 0
function _init()
  if stat(6)=="headless" then
    extcmd("set_filename", "sfx-loop")
    extcmd("audio_rec")
  end
  sfx(0, 0)
end
function tick()
  t += 1
  if t == 48 then sfx(-2, 0) end
  if stat(6)=="headless" and t == 64 then
    extcmd("audio_end", 1)
    extcmd("shutdown")
  end
end
function _draw()
  cls(1)
  print("sfx-loop "..t, 4, 4, 7)
end
function _update() tick() end
function _update60() tick() end
__sfx__
000801080d0700f070110701207014070160701807019070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
