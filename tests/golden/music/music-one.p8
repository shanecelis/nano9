pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- golden: music-one
local t = 0
function _init()
  if stat(6)=="headless" then
    extcmd("set_filename", "music-one")
    extcmd("audio_rec")
  end
  music(0)
end
function tick()
  t += 1
  if stat(6)=="headless" and t == 32 then
    extcmd("audio_end", 1)
    extcmd("shutdown")
  end
end
function _draw()
  cls(1)
  print("music-one "..t, 4, 4, 7)
end
function _update() tick() end
function _update60() tick() end
__sfx__
000800000d0700f070110701207014070160701807019070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
__music__
00 00414243
