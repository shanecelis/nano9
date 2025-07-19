-- Advanced Micro Platformer
-- Original code 2017 (c) Matt Hughson
-- Modified code 2023 (c) Shane Celis
-- License CC4-BY-NC-SA

-- If you make a game with this starter kit, please consider linking back to the
-- [bbs post](https://www.lexaloffle.com/bbs/?tid=28793) for this cart, so that
-- others can learn from it too! Enjoy!
--
-- @matthughson


-- _init the game to its initial state.
function _init()
  p1=jumper:new { x=44, y=100 }
  p1:set_anim("walk")
  cam=dolly:new(nil, p1)
end

--p8 functions
--------------------------------

function _update()
  p1:update()
  cam:update()
  -- demo camera shake
  if btnp(4) then cam:shake(15,2) end
  end

function _draw()

  cls(0)

  camera(cam:cam_pos())

  map(0,0,0,0,128,128)

  p1:draw()

  --hud
  camera(0,0)

  print("adv. micro platformer",24,4,7,0,0)

end
