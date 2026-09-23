pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- golden: default font glyphs
local frame = 0
function _draw()
  frame += 1
  cls(0)
  print("hello world", 0, 0, 7)
  print("HELLO WORLD", 0, 8, 7)
  print("0123456789", 0, 16, 7)
  print("the quick brown", 0, 24, 12)
  print("fox jumps over", 0, 32, 12)
  print("lazy dog.", 0, 40, 8)
  print("!\"#$%&'()*+,-./", 0, 56, 7)
  print(":;<=>?@[\\]^_`", 0, 64, 7)
  print("{|}~", 0, 72, 7)
  -- Text layout lands a frame after the first print.
  if stat(6)=="headless" and frame > 2 then
    extcmd("set_filename", "font")
    extcmd("screen", 1, 1)
    extcmd("shutdown")
  end
end
