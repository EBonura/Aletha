-- ashen edge
-- color 14 = transparent

--##generated##
-- (replaced by build.py)
-- char_base, cell_w, cell_h, trans
-- anim indices, anchor data
--##end##

-- combo chain: z press cycles
-- attack1 -> cross_slice
combo_chain={a_atk1,a_xslice}

-- anim speeds (frames per anim frame)
aspd={}
aspd[a_idle]=6
aspd[a_run]=5
aspd[a_jump]=5
aspd[a_fall]=5
aspd[a_hit]=5
aspd[a_land]=2
aspd[a_atk1]=6
aspd[a_xslice]=6
aspd[a_sweep]=4
aspd[a_death]=6

-- physics
ground_y=100
grav=0.3
jump_spd=-5
run_spd=1.5
friction=0.8

-- collision box (relative to px,py)
-- px = body center x, py = top of sprite
-- box: x from px+cb_x0 to px+cb_x1
--      y from py+cb_y0 to py+cb_y1
cb_x0=-3  -- left edge offset from px
cb_x1=3   -- right edge offset from px
cb_y0=3   -- top edge offset from py (skip hair)
cb_y1=19  -- bottom edge = py+cell_h
debug_box=false

-- player state
px=64 py=ground_y
vx=0 vy=0
facing=1
grounded=true

-- animation state
state="idle"
-- states: idle,run,jump,fall,land,
--  attack,sweep,interact
cur_anim=a_idle
cur_frame=1
anim_timer=0

-- combo state
combo_idx=0
combo_queued=false

-- input buffer (frames remaining)
buf_jump=0
buf_atk=0 -- z buffered
buf_sweep=0 -- x buffered
buf_dur=8 -- buffer window (frames)

-- attack movement tracking
atk_anchor0=0 -- anchor at start of attack anim
air_atk=false -- true if attack started in air

-- forward push on attack end
atk_push={}
atk_push[a_atk1]=4
atk_push[a_xslice]=6
atk_push[a_sweep]=0 -- sweep uses anchor drift

-- -- decoders --

function decode_rle(off,npix)
 local buf={}
 local idx=1
 while idx<=npix do
  local b=peek(off)
  off+=1
  buf[idx]=b\16
  local cnt=b&15
  for c=1,cnt do
   buf[idx+c]=buf[idx]
  end
  idx+=cnt+1
 end
 return buf
end

function decode_skip(buf,off)
 local nd=peek(off)
 off+=1
 if nd==255 then
  nd=peek(off)+peek(off+1)*256
  off+=2
 end
 if nd==0 then return end
 local pos=peek(off)+peek(off+1)*256+1
 off+=2
 buf[pos]=peek(off)
 off+=1
 for i=2,nd do
  local b=peek(off)
  off+=1
  local skip=b\16
  local col=b&15
  if skip==15 then
   local ext=peek(off)
   off+=1
   if ext==255 then
    skip=peek(off)+peek(off+1)*256
    off+=2
   else
    skip=15+ext
   end
  end
  pos+=skip+1
  buf[pos]=col
 end
end

function read_anim(a)
 local cb=char_base
 local na=peek(cb)
 local aoff=peek(cb+3+(a-1)*2)+peek(cb+3+(a-1)*2+1)*256
 local ab=cb+3+na*2+aoff
 local nf=peek(ab)
 local enc=peek(ab+1)
 if enc==0 then
  local nk=peek(ab+2)
  local bx=peek(ab+3)
  local by=peek(ab+4)
  local bw=peek(ab+5)
  local bh=peek(ab+6)
  local ki_off=ab+7
  local ks_off=ki_off+nk
  local as_off=ks_off+nk*2
  local do_off=as_off+nf
  local data_off=do_off+nf*2
  local ksz={}
  for i=0,nk-1 do
   ksz[i]=peek(ks_off+i*2)+peek(ks_off+i*2+1)*256
  end
  return {
   enc=0,nf=nf,nk=nk,
   bx=bx,by=by,bw=bw,bh=bh,
   ki_off=ki_off,ks_off=ks_off,
   as_off=as_off,do_off=do_off,
   data_off=data_off,ksz=ksz
  }
 else
  local fo_off=ab+2
  local data_off=fo_off+nf*2
  return {
   enc=1,nf=nf,
   fo_off=fo_off,data_off=data_off
  }
 end
end

acache={}

function cache_anims()
 -- fully decode all frames into lua
 -- tables so sprite sheet memory can
 -- be overwritten by tile data later
 local na=peek(char_base)
 for a=1,na do
  local ai=read_anim(a)
  local frames={}
  if ai.enc==0 then
   local npix=ai.bw*ai.bh
   -- decode keyframes
   local kbufs={}
   local koff=ai.data_off
   for i=0,ai.nk-1 do
    kbufs[i]=decode_rle(koff,npix)
    koff+=ai.ksz[i]
   end
   -- decode every frame now
   for f=1,ai.nf do
    local ki=peek(ai.as_off+f-1)
    local buf={}
    local kb=kbufs[ki]
    for i=1,#kb do buf[i]=kb[i] end
    local doff=peek(ai.do_off+(f-1)*2)+peek(ai.do_off+(f-1)*2+1)*256
    decode_skip(buf,ai.data_off+doff)
    frames[f]={buf,ai.bx,ai.by,ai.bw,ai.bh}
   end
  else
   -- type 1: decode each frame
   for f=1,ai.nf do
    local foff=peek(ai.fo_off+(f-1)*2)+peek(ai.fo_off+(f-1)*2+1)*256
    local addr=ai.data_off+foff
    local bx=peek(addr)
    local by=peek(addr+1)
    local bw=peek(addr+2)
    local bh=peek(addr+3)
    if bw==0 or bh==0 then
     frames[f]={{},0,0,0,0}
    else
     local buf=decode_rle(addr+4,bw*bh)
     frames[f]={buf,bx,by,bw,bh}
    end
   end
  end
  acache[a]={ai=ai,frames=frames}
 end
end

function get_frame(a,f)
 local fr=acache[a].frames[f]
 return fr[1],fr[2],fr[3],fr[4],fr[5]
end

function draw_char(a,f,sx,sy,flip)
 local buf,bx,by,bw,bh=get_frame(a,f)
 if bw==0 then return end
 local idx=1
 for y=0,bh-1 do
  for x=0,bw-1 do
   local col=buf[idx]
   if col~=trans then
    local dx
    if flip then
     dx=cell_w-1-bx-x
    else
     dx=bx+x
    end
    pset(sx+dx,sy+by+y,col)
   end
   idx+=1
  end
 end
end

-- -- animation helpers --

function set_anim(a)
 if cur_anim~=a then
  cur_anim=a
  cur_frame=1
  anim_timer=0
 end
end

function anim_nf()
 return acache[cur_anim].ai.nf
end

function anim_done()
 return cur_frame>=anim_nf()
  and anim_timer>=aspd[cur_anim]-1
end

function tick_anim()
 anim_timer+=1
 if anim_timer>=aspd[cur_anim] then
  anim_timer=0
  if cur_frame<anim_nf() then
   cur_frame+=1
  elseif state=="idle" or state=="run" then
   cur_frame=1
  end
 end
end

function in_combo_window()
 -- accept input in last 40% of anim
 local nf=anim_nf()
 return cur_frame>=ceil(nf*0.6)
end

-- -- tile/map system --

-- defaults (overridden by generated block when level data exists)
if not map_base then map_base=0x2000 end
if not lvl_w then lvl_w=8 end
if not lvl_h then lvl_h=8 end
if not lvl_nt then lvl_nt=0 end
if not spn_x then spn_x=0 end
if not spn_y then spn_y=0 end
if not map_in_str then map_in_str=false end
if not tflg then tflg={} end

-- map buffer: mdat[y*lvl_w+x+1]=tile_id (0=empty, 1-N=runtime tile)
mdat={}
cam_x=0
cam_y=0

function load_tiles()
 if lvl_nt==0 then
  for i=1,lvl_w*lvl_h do mdat[i]=0 end
  return
 end
 -- read header from __map__ (0x2000)
 local b=map_base
 local nt=peek(b)
 local mw=peek(b+1)+peek(b+2)*256
 local mh=peek(b+3)+peek(b+4)*256
 local sx=peek(b+5)+peek(b+6)*256
 local sy=peek(b+7)+peek(b+8)*256
 local tdsz=peek(b+9)+peek(b+10)*256

 -- poke tile pixel data into sprite sheet
 -- each tile = 128 bytes, laid out in 8-tile-wide grid
 -- tile N goes to sprite sheet position:
 --   col = N % 8, row = N \ 8
 --   pixel (col*16, row*16) = byte offset (row*16*64 + col*8)
 local tdata=b+11
 for t=0,nt-1 do
  local tcol=t%8
  local trow=t\8
  -- copy 16 rows of 8 bytes each into sprite sheet
  for py=0,15 do
   local src=tdata+t*128+py*8
   local dst=(trow*16+py)*64+tcol*8
   for i=0,7 do
    poke(dst+i,peek(src+i))
   end
  end
 end

 -- decompress map RLE
 local rle_off=tdata+tdsz
 local rle_src="mem" -- read from memory
 if map_in_str then
  rle_src="str"
 end

 local mx,my=0,0
 mdat={}
 for i=1,mw*mh do mdat[i]=0 end

 if rle_src=="mem" then
  local p=rle_off
  while my<mh do
   local cell=peek(p)
   local run=peek(p+1)
   p+=2
   for i=1,run do
    if my<mh then
     mdat[my*mw+mx+1]=cell
     mx+=1
     if mx>=mw then mx=0 my+=1 end
    end
   end
  end
 else
  -- read from lua string
  local p=1
  while my<mh and p<#map_rle_str do
   local cell=ord(map_rle_str,p)
   local run=ord(map_rle_str,p+1)
   p+=2
   for i=1,run do
    if my<mh then
     mdat[my*mw+mx+1]=cell
     mx+=1
     if mx>=mw then mx=0 my+=1 end
    end
   end
  end
 end
end

function tile_at(tx,ty)
 if tx<0 or tx>=lvl_w or ty<0 or ty>=lvl_h then return 0 end
 return mdat[ty*lvl_w+tx+1]
end

function tile_flag(tx,ty)
 local t=tile_at(tx,ty)
 if t==0 then return 0 end
 return tflg[t] or 0
end

function tile_solid(tx,ty)
 -- flag bit 0 = solid
 return band(tile_flag(tx,ty),1)>0
end

-- check if collision box overlaps
-- any solid tile
function box_hits_solid(bx0,by0,bx1,by1)
 local tx0=flr(bx0/16)
 local ty0=flr(by0/16)
 local tx1=flr((bx1-0.01)/16)
 local ty1=flr((by1-0.01)/16)
 for ty=ty0,ty1 do
  for tx=tx0,tx1 do
   if tile_solid(tx,ty) then
    return true
   end
  end
 end
 return false
end

function resolve_x()
 local bx0=px+cb_x0
 local by0=py+cb_y0
 local bx1=px+cb_x1
 local by1=py+cb_y1
 if not box_hits_solid(bx0,by0,bx1,by1) then
  return
 end
 if vx>0 then
  -- moving right: snap left edge of
  -- colliding tile column
  local tx1=flr((bx1-0.01)/16)
  px=tx1*16-cb_x1
  vx=0
 elseif vx<0 then
  -- moving left: snap to right edge
  -- of colliding tile column
  local tx0=flr(bx0/16)
  px=(tx0+1)*16-cb_x0
  vx=0
 end
end

function resolve_y()
 local bx0=px+cb_x0
 local by0=py+cb_y0
 local bx1=px+cb_x1
 local by1=py+cb_y1
 if not box_hits_solid(bx0,by0,bx1,by1) then
  grounded=false
  return
 end
 if vy>=0 then
  -- falling/standing: snap to top
  local ty1=flr((by1-0.01)/16)
  py=ty1*16-cb_y1
  vy=0
  if not grounded then
   grounded=true
   if state=="fall" then
    set_anim(a_land)
    state="land"
   end
  end
 elseif vy<0 then
  -- jumping up: hit ceiling
  local ty0=flr(by0/16)
  py=(ty0+1)*16-cb_y0
  vy=0
 end
end

function draw_map()
 -- compute visible tile range
 local ts=16 -- tile size in pixels
 local tx0=flr(cam_x/ts)
 local ty0=flr(cam_y/ts)
 local tx1=tx0+8 -- 128/16 = 8 tiles wide
 local ty1=ty0+8
 -- clamp
 tx0=max(0,tx0)
 ty0=max(0,ty0)
 tx1=min(lvl_w-1,tx1)
 ty1=min(lvl_h-1,ty1)

 for ty=ty0,ty1 do
  for tx=tx0,tx1 do
   local t=mdat[ty*lvl_w+tx+1]
   if t>0 then
    -- tile t is at sprite sheet position:
    -- col=(t-1)%8, row=(t-1)\8
    -- spr id = row*2*16 + col*2
    local sc=(t-1)%8
    local sr=(t-1)\8
    local sid=sr*32+sc*2
    spr(sid,tx*ts-cam_x,ty*ts-cam_y,2,2)
   end
  end
 end
end

function get_visual_x()
 -- during attack/sweep, body drifts
 -- from anchor. return visual center.
 if state=="attack" or state=="sweep" then
  local ax=anc[cur_anim][cur_frame]
  local drift=ax-atk_anchor0
  return px+drift*facing
 end
 return px
end

function update_camera()
 local target_x=get_visual_x()-64
 local target_y=py-64
 -- clamp to map bounds
 target_x=mid(0,target_x,lvl_w*16-128)
 target_y=mid(0,target_y,lvl_h*16-128)
 -- smooth follow
 cam_x+=(target_x-cam_x)*0.15
 cam_y+=(target_y-cam_y)*0.15
end

-- -- game --

function _init()
 palt(0,false)
 palt(trans,true)
 cache_anims()
 -- load tiles into sprite sheet (overwrites __gfx__)
 load_tiles()
 -- set player to spawn
 if spn_x then
  px=spn_x*16+8
  py=spn_y*16
 end
 -- set ground_y to bottom of map for now
 ground_y=lvl_h*16
end

function do_jump()
 vy=jump_spd
 grounded=false
 set_anim(a_jump)
 state="jump"
 buf_jump=0
end

function start_combo()
 combo_idx=1
 combo_queued=false
 vx=0
 set_anim(combo_chain[1])
 atk_anchor0=anc[combo_chain[1]][1]
 state="attack"
 buf_atk=0
end

function start_sweep()
 vx=0
 set_anim(a_sweep)
 atk_anchor0=anc[a_sweep][1]
 state="sweep"
 air_atk=false
 buf_sweep=0
end

function start_air_atk()
 air_atk=true
 -- force reset even if same anim
 cur_anim=a_xslice
 cur_frame=1
 anim_timer=0
 atk_anchor0=anc[a_xslice][1]
 state="sweep"
 buf_atk=0
 buf_sweep=0
end

function end_attack()
 local drift=anc[cur_anim][cur_frame]
  -atk_anchor0
 px+=drift*facing
 local push=atk_push[cur_anim] or 0
 px+=push*facing
end

function air_control(lr)
 if lr~=0 then
  vx=lr*run_spd
  facing=lr
 else
  vx*=friction
  if abs(vx)<0.1 then vx=0 end
 end
end

function _update60()
 local lr=0
 if btn(0) then lr=-1 end
 if btn(1) then lr=1 end

 -- -- input buffering --
 if btnp(2) then buf_jump=buf_dur end
 if btnp(4) then buf_atk=buf_dur end
 if btnp(5) then buf_sweep=buf_dur end
 if buf_jump>0 then buf_jump-=1 end
 if buf_atk>0 then buf_atk-=1 end
 if buf_sweep>0 then buf_sweep-=1 end

 -- -- state machine --
 if state=="idle" or state=="run" then
  -- buffered jump
  if buf_jump>0 and grounded then
   do_jump()
  -- buffered z: start combo
  elseif buf_atk>0 then
   start_combo()
  -- buffered x: sweep
  elseif buf_sweep>0 then
   start_sweep()
  -- down: interact
  elseif btnp(3) then
   -- interact placeholder
   -- state="interact"
  -- movement
  elseif lr~=0 then
   vx=lr*run_spd
   facing=lr
   if state~="run" then
    set_anim(a_run)
    state="run"
   end
  else
   vx*=friction
   if abs(vx)<0.1 then vx=0 end
   if state~="idle" then
    set_anim(a_idle)
    state="idle"
   end
  end

 elseif state=="jump" then
  air_control(lr)
  if buf_atk>0 or buf_sweep>0 then
   start_air_atk()
  elseif vy>=0 then
   set_anim(a_fall)
   state="fall"
  end

 elseif state=="fall" then
  air_control(lr)
  if buf_atk>0 or buf_sweep>0 then
   start_air_atk()
  elseif grounded then
   -- check buffers before landing
   if buf_jump>0 then
    do_jump()
   else
    vx=0
    set_anim(a_land)
    state="land"
   end
  end

 elseif state=="land" then
  vx=0
  if anim_done() then
   -- check buffered actions
   if buf_atk>0 then
    start_combo()
   elseif buf_sweep>0 then
    start_sweep()
   else
    set_anim(a_idle)
    state="idle"
   end
  end

 elseif state=="attack" then
  -- buffer combo input anytime
  if buf_atk>0
   and combo_idx<#combo_chain then
   combo_queued=true
   buf_atk=0
  end
  if anim_done() then
   end_attack()
   if combo_queued
    and combo_idx<#combo_chain then
    combo_idx+=1
    combo_queued=false
    set_anim(combo_chain[combo_idx])
    atk_anchor0=anc[combo_chain[combo_idx]][1]
   else
    combo_idx=0
    set_anim(a_idle)
    state="idle"
   end
  end

 elseif state=="sweep" then
  -- air atk: allow re-trigger
  if air_atk then
   air_control(lr)
   if (buf_atk>0 or buf_sweep>0)
    and anim_done() then
    start_air_atk()
    return
   end
  end
  if anim_done() then
   if not air_atk then
    end_attack()
   end
   if air_atk and not grounded then
    air_atk=false
    set_anim(a_fall)
    state="fall"
   elseif air_atk and grounded then
    air_atk=false
    set_anim(a_land)
    state="land"
   else
    -- sweep -> cross_slice chain
    if buf_atk>0 or buf_sweep>0 then
     air_atk=false
     vx=0
     set_anim(a_xslice)
     atk_anchor0=anc[a_xslice][1]
     state="attack"
     buf_atk=0
     buf_sweep=0
     combo_idx=0
    else
     air_atk=false
     set_anim(a_idle)
     state="idle"
    end
   end
  end
  -- air sweep: land during anim
  if air_atk and grounded then
   air_atk=false
   set_anim(a_land)
   state="land"
  end

 --elseif state=="interact" then
 -- placeholder for future interaction
 -- if anim_done() then
 --  state="idle"
 --  set_anim(a_idle)
 -- end
 end

 -- -- physics --
 if not grounded then
  vy+=grav
 end

 -- move X, then resolve X collisions
 px+=vx
 resolve_x()

 -- move Y, then resolve Y collisions
 py+=vy
 local was_grounded=grounded
 resolve_y()

 -- fallback: bottom of map
 if py+cb_y1>=lvl_h*16 then
  py=lvl_h*16-cb_y1
  vy=0
  if not grounded then
   grounded=true
   if state=="fall" then
    set_anim(a_land)
    state="land"
   end
  end
 end

 -- clamp to map bounds
 local min_x=-cb_x0
 local max_x=lvl_w*16-1-cb_x1
 px=mid(min_x,px,max_x)

 -- update camera
 update_camera()

 -- tick animation
 tick_anim()
end

function _draw()
 cls(1)

 -- draw map
 draw_map()

 -- draw player anchored to body center
 local ax=anc[cur_anim][cur_frame]
 local flip=facing==-1
 local dx
 if state=="attack" or state=="sweep" then
  local drift=ax-atk_anchor0
  if flip then
   dx=px-drift-(cell_w-1-ax)
  else
   dx=px+drift-ax
  end
 else
  if flip then
   dx=px-(cell_w-1-ax)
  else
   dx=px-ax
  end
 end
 draw_char(cur_anim,cur_frame,dx-cam_x,py-cam_y,flip)

 -- debug: draw collision box
 if debug_box then
  rect(px+cb_x0-cam_x,py+cb_y0-cam_y,
       px+cb_x1-1-cam_x,py+cb_y1-1-cam_y,8)
 end

 -- hud
 print(state,1,1,7)
 if state=="attack" then
  print("combo:"..combo_idx.."/"
   ..#combo_chain,1,8,7)
 end
 if combo_queued then
  print("queued",1,15,11)
 end
end
