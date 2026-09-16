pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- golden: sfx-waves
local t = 0
function _init()
  if stat(6)=="headless" then
    extcmd("set_filename", "sfx-waves")
    extcmd("audio_rec")
  end
  sfx(0)
end
function tick()
  t += 1
  if stat(6)=="headless" and t == 80 then
    extcmd("audio_end", 1)
    extcmd("shutdown")
  end
end
function _draw()
  cls(1)
  print("sfx-waves "..t, 4, 4, 7)
end
function _update() tick() end
function _update60() tick() end
__sfx__
001000001907019170192701937019470195701967019770000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
