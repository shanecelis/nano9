pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
--advanced micro platformer
--original code 2017 (c) matt hughson
--modified code 2023 (c) shane celis
--license mit

--#include actor.p8

jumper = actor:new {

  dx=0, dy=0,
  max_dx=1,--max x speed
  max_dy=2,--max y speed

  jump_speed=-1.75,--jump veloclity
  acc=0.05,--acceleration
  dcc=0.8,--decceleration
  air_dcc=1,--air decceleration
  gravity=0.15,
  jump_snd=0,


  --helper for more complex
  --button press tracking.
  --todo: generalize button index.
  jump_button=
    {
      update=function(self, jumpbtn)
        --start with assumption
        --that not a new press.
        -- self.is_pressed=false
        -- self.is_pressed=btnp(5)
        self.is_down=jumpbtn
        if self.is_down then
          self.ticks_down+=1
        else
          self.ticks_down=0
        end
      end,
      --state
      is_down=false,--currently down
      ticks_down=0,--how long down
    },

  jump_hold_time=0,--how long jump is held
  min_jump_press=5,--min time jump can be held
  max_jump_press=16,--max time jump can be held

  --jump_btn_released=true,--can we jump again?
  grounded=false,--on ground

  airtime=0,--time since grounded

  --animation definitions.
  --use with set_anim()
  anims=
    {
      stand=
        {
          ticks=1,--how long is each frame shown.
          frames={2},--what frames are shown.
        },
      walk=
        {
          ticks=5,
          frames={3,4,5,6},
        },
      jump=
        {
          ticks=1,
          frames={1},
        },
      slide=
        {
          ticks=1,
          frames={7},
        },
    },

  curanim="walk",--currently playing animation
  curframe=1,--curent frame of animation.
  animtick=0,--ticks until next frame should show.
  flipx=false,--show sprite be flipped.

  --request new animation to play.
  set_anim=function(self,anim)
    if (anim==self.curanim) return--early out.
    self.animtick=self.anims[anim].ticks--ticks count down.
    self.curanim=anim
    self.curframe=1
  end,
  
  update=function(self)
    local bl=btn(0) --left
    local br=btn(1) --right
    self:move(bl,br, btn(5))
  end,

  --call once per tick.
  move=function(self, bl, br, jump)

    --todo: kill enemies.

    --track button presses
    

    --move left/right
    if bl then
      self.dx-=self.acc
      br=false--handle double press
    elseif br then
      self.dx+=self.acc
    else
      if self.grounded then
        self.dx*=self.dcc
      else
        self.dx*=self.air_dcc
      end
    end

    --limit walk speed
    self.dx=mid(-self.max_dx,self.dx,self.max_dx)

    --move in x
    self.x+=self.dx

    --hit walls
    self:collide_side()

    --jump buttons
    self.jump_button:update(jump)

    --jump is complex.
    --we allow jump if:
    --  on ground
    --  recently on ground
    --  pressed btn right before landing
    --also, jump velocity is
    --not instant. it applies over
    --multiple frames.
    if self.jump_button.is_down then
      --is player on ground recently.
      --allow for jump right after
      --walking off ledge.
      local on_ground=self.grounded or self.airtime<self.min_jump_press
      --was btn presses recently?
      --allow for pressing right before
      --hitting ground.
      local new_jump_btn=self.jump_button.ticks_down<10
      --is player continuing a jump
      --or starting a new one?
      if self.jump_hold_time>0 or (on_ground and new_jump_btn) then
        if(self.jump_hold_time==0)sfx(jump_snd)--new jump snd
        self.jump_hold_time+=1
        --keep applying jump velocity
        --until max jump time.
        if self.jump_hold_time<self.max_jump_press then
          self.dy=self.jump_speed--keep going up while held
        end
      end
    else
      self.jump_hold_time=0
    end

    --move in y
    self.dy+=self.gravity
    self.dy=mid(-self.max_dy,self.dy,self.max_dy)
    self.y+=self.dy

    --floor
    if not self:collide_floor() then
      self:set_anim("jump")
      self.grounded=false
      self.airtime+=1
    end

    --roof
    self:collide_roof()

    --handle playing correct animation when
    --on the ground.
    if self.grounded then
      if br then
        --pressing right but still moving left.
        self:set_anim(self.dx<0 and "slide" or "walk")
      elseif bl then
        --pressing left but still moving right.
        self:set_anim(self.dx>0 and "slide" or "walk")
      else
        self:set_anim("stand")
      end
    end

    --flip
    if br then
      self.flipx=false
    elseif bl then
      self.flipx=true
    end

    --anim tick
    self.animtick-=1
    if self.animtick<=0 then
      local a=self.anims[self.curanim]
      self.curframe=self.curframe % #a.frames + 1
      self.animtick=a.ticks -- init timer
    end

  end,

  --draw the player
  draw=function(self)
    local a=self.anims[self.curanim]
    local frame=a.frames[self.curframe]
    spr(frame,
        self.x,
        self.y,
        self.w/8,
        self.h/8,
        self.flipx,
        false)
  end,

  --check if pushing into side tile and resolve.
  --requires self.dx,self.x,self.y, and
  --assumes tile flag 0 == solid
  --assumes sprite size of 8x8
  collide_side = function(self)
    -- for   i in all({self.h/6, self.h*5/6}) do
    for   i in all({self.h/6, self.h*5/6}) do
      local j=self.w*5/6
      if self.dx > 0 then -- going right
        if fget(mget((self.x+j)/8,(self.y+i)/8),0) then
          self.dx=0
          self.x=flr((self.x+j)/8)*8-1.01*j -- ugh!
          return true
        end
      elseif self.dx < 0 then -- going left
        j = self.w/6
        if fget(mget((self.x+j)/8,(self.y+i)/8),0) then
          self.dx=0
          self.x=flr((self.x+j)/8)*8+self.w-j
          return true
        end
      end
    end
    --didn't hit a solid tile.
    return false
  end,

  --check if pushing into floor tile and resolve.
  --requires self.dx,self.x,self.y,self.grounded,self.airtime and
  --assumes tile flag 0 or 1 == solid
  collide_floor = function (self)
    --only check for ground when falling.
    if (self.dy<=0) return false
    local landed=false
    --check for collision at multiple points along the bottom
    --of the sprite: left, center, and right.
    --
    for i in all({self.w/6,self.w/2,self.w*5/6}) do
      local tile=mget((self.x+i)/8,(self.y+self.h)/8)
      if fget(tile,0) or (fget(tile,1) and self.dy>=0) then
        self.dy=0
        self.y=flr((self.y+self.h)/8)*8-self.h
        self.grounded=true
        self.airtime=0
        landed=true
      end
    end
    return landed
  end,

  --check if pushing into roof tile and resolve.
  --requires self.dy,self.x,self.y, and
  --assumes tile flag 0 == solid
  collide_roof = function (self)
    --check for collision at multiple points along the top
    --of the sprite: left, center, and right.
    for i in all({self.h/6,self.h*5/6}) do
      if fget(mget((self.x+i)/8,self.y/8),0) then
        self.dy=0
        self.y=flr((self.y-self.h)/8)*8+8+self.h
        self.jump_hold_time=0
      end
    end
  end
}

function std_anim(start, walk_speed)
  return {
    stand= {
      ticks=1,--how long is each frame shown.
      frames={start+1},--what frames are shown.
    },
    walk= {
      ticks=walk_speed or 5,
      frames={start+2,start+3,start+4,start+5},
    },
    jump= {
      ticks=1,
      frames={start},
    },
    slide= {
      ticks=1,
      frames={start+6},
    },
  }
end
