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

-- game state: 0=title, 1=play
gs=0

-- anim speeds (frames per anim frame)
-- order: idle,run,jump,fall,hit,land,atk1,xslice,sweep,death,door,sw_start,sw_idle,sw_down,title
aspd=split"6,5,5,5,5,2,6,6,4,6,6,6,6,8,30"

-- physics
grav,max_fall,jump_spd,run_spd,friction=0.15,3,-3.5,1.5,0.8

-- collision box (relative to px,py)
-- box: x from px-3 to px+3, y from py+3 to py+19
cb_x0,cb_x1,cb_y0,cb_y1=-3,3,3,19

-- player state
px,py,vx,vy=64,0,0,0
facing,grounded,prev_by1,air_time=1,true,0,0

-- animation state
state="idle"
cur_anim,cur_frame,anim_timer=a_idle,1,0

-- combo state
combo_idx,combo_queued=0,false

-- input buffer (frames remaining)
buf_jump,buf_atk,buf_sweep=0,0,0
buf_dur=8

-- attack movement tracking
atk_anchor0,atk_px0,air_atk=0,0,false

-- forward push on attack end
-- order: idle,run,jump,fall,hit,land,atk1,xslice,sweep,death
atk_push=split"0,0,0,0,0,0,4,6,0,0"

-- particles & collectibles
parts={} -- particle pool
gems=0   -- collected count

-- entity system
ents={}
ent_grp={}

-- -- decoders --

function pk2(a)
 return peek(a)+peek(a+1)*256
end

function decode_rle(off,npix)
 local buf={}
 local idx=1
 while idx<=npix do
  local b=peek(off)
  off+=1
  buf[idx]=b\16
  local cnt=b&15
  if cnt==15 then
   cnt=15+peek(off)
   off+=1
  end
  for c=1,cnt do
   buf[idx+c]=buf[idx]
  end
  idx+=cnt+1
 end
 return buf,off
end

function decode_rle2(off,npix,pal,nbits)
 local buf={} local idx=1
 local sh=8-nbits
 local xv=(1<<sh)-1
 while idx<=npix do
  local b=peek(off) off+=1
  local ci=b>>sh
  local cf=band(b,xv)
  local cnt
  if cf==xv then
   cnt=xv+1+peek(off) off+=1
  else
   cnt=cf+1
  end
  local col=pal[ci]
  for i=0,cnt-1 do buf[idx+i]=col end
  idx+=cnt
 end
 return buf
end

function decode_skip(buf,off)
 local nd=peek(off)
 off+=1
 if nd==255 then
  nd=pk2(off)
  off+=2
 end
 if nd==0 then return end
 local pos=pk2(off)+1
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
    skip=pk2(off)
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
 local aoff=pk2(cb+3+(a-1)*2)
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
   ksz[i]=pk2(ks_off+i*2)
  end
  return {
   enc=0,nf=nf,nk=nk,
   bx=bx,by=by,bw=bw,bh=bh,
   ki_off=ki_off,ks_off=ks_off,
   as_off=as_off,do_off=do_off,
   data_off=data_off,ksz=ksz
  }
 elseif enc==1 then
  local fo_off=ab+2
  local data_off=fo_off+nf*2
  return {
   enc=1,nf=nf,
   fo_off=fo_off,data_off=data_off
  }
 elseif enc==2 then
  -- enc==2: palette + bit-packed RLE
  local nbits=peek(ab+2)
  local nc=peek(ab+3)
  local pal={}
  for i=0,nc-1 do pal[i]=peek(ab+4+i) end
  local fo_off=ab+4+nc
  local data_off=fo_off+nf*2
  return {
   enc=2,nf=nf,nbits=nbits,pal=pal,
   fo_off=fo_off,data_off=data_off
  }
 else
  -- enc==3: tile dedup (single frame)
  return {enc=3,nf=1,ab=ab}
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
    local doff=pk2(ai.do_off+(f-1)*2)
    decode_skip(buf,ai.data_off+doff)
    frames[f]={buf,ai.bx,ai.by,ai.bw,ai.bh}
   end
  elseif ai.enc==1 then
   -- type 1: per-frame 4bpp RLE
   for f=1,ai.nf do
    local foff=pk2(ai.fo_off+(f-1)*2)
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
  elseif ai.enc==2 then
   -- type 2: palette + bit-packed RLE
   for f=1,ai.nf do
    local foff=pk2(ai.fo_off+(f-1)*2)
    local addr=ai.data_off+foff
    local bx=peek(addr)
    local by=peek(addr+1)
    local bw=peek(addr+2)
    local bh=peek(addr+3)
    if bw==0 or bh==0 then
     frames[f]={{},0,0,0,0}
    else
     local buf=decode_rle2(addr+4,bw*bh,ai.pal,ai.nbits)
     frames[f]={buf,bx,by,bw,bh}
    end
   end
  else
   -- type 3: tile dedup
   local p=ai.ab+2
   local tw=peek(p) local th=peek(p+1)
   local tcols=peek(p+2) local trows=peek(p+3)
   local nbits=peek(p+4) local nc=peek(p+5) p+=6
   local pal={} for i=0,nc-1 do pal[i]=peek(p+i) end p+=nc
   local nu=peek(p) local tdsz=pk2(p+1) p+=3
   local to_base=p p+=nu*2
   local td_base=p p+=tdsz
   local im_base=p
   local tpix=tw*th
   local tiles={}
   for i=0,nu-1 do
    local off=pk2(to_base+i*2)
    tiles[i]=decode_rle2(td_base+off,tpix,pal,nbits)
   end
   local iw=tcols*tw local ih=trows*th
   local buf={}
   for tr=0,trows-1 do
    for tc=0,tcols-1 do
     local ti=peek(im_base+tr*tcols+tc)
     local tile=tiles[ti]
     for ty=0,th-1 do
      for tx=0,tw-1 do
       buf[(tr*th+ty)*iw+tc*tw+tx+1]=tile[ty*tw+tx+1]
      end
     end
    end
   end
   frames[1]={buf,0,0,iw,ih}
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

-- -- tile/map system --

-- map buffers: mdat[layer][y*lvl_w+x+1]
-- layer 1=bg, 2=main, 3=fg
mdat={}
cam_x=0
cam_y=0

function load_tiles()
 local sz=lvl_w*lvl_h
 if lvl_nt==0 then
  for L=1,lvl_nl do
   mdat[L]={}
   for i=1,sz do mdat[L][i]=0 end
  end
  return
 end
 -- header: 12+nl bytes at map_base
 local b=map_base
 local nt=peek(b)
 local nl=peek(b+1)
 local mw=pk2(b+2)
 local mh=pk2(b+4)
 local sx=pk2(b+6)
 local sy=pk2(b+8)
 local tbsz=pk2(b+10)
 -- per-layer encoding modes
 local lmode={}
 for i=1,nl do lmode[i]=peek(b+11+i) end

 -- tile blob starts after header+mode bytes
 local tblob=b+12+nl

 -- decode ALL data from __map__ into
 -- lua tables before writing to memory

 -- 1. decode all tile pixels as single blob
 local all_pix,p=decode_rle(tblob,nt*256)
 local tile_pix={}
 for t=0,nt-1 do
  tile_pix[t]={}
  local base=t*256
  for i=1,256 do
   tile_pix[t][i]=all_pix[base+i]
  end
 end

 -- 2. decode each layer's map data
 for L=1,nl do
  mdat[L]={}
  for i=1,sz do mdat[L][i]=0 end

  if lmode[L]==0 then
   -- mode 0: standard RLE
   local mx,my=0,0
   while my<mh do
    local cell=peek(p)
    local run=peek(p+1)
    p+=2
    for i=1,run do
     if my<mh then
      mdat[L][my*mw+mx+1]=cell
      mx+=1
      if mx>=mw then mx=0 my+=1 end
     end
    end
   end

  elseif lmode[L]==1 then
   -- mode 1: tiled fill
   local tw=peek(p)
   local th=peek(p+1)
   local dx=peek(p+2)
   local dy=peek(p+3)
   local rw=peek(p+4)
   local rh=peek(p+5)
   local td=p+6
   for ry=0,rh-1 do
    for rx=0,rw-1 do
     local tc=peek(td+(ry%th)*tw+(rx%tw))
     mdat[L][(dy+ry)*mw+dx+rx+1]=tc
    end
   end
   p+=6+tw*th

  elseif lmode[L]==2 then
   -- mode 2: packbits
   local idx=1
   while idx<=sz do
    local ctrl=peek(p)
    p+=1
    if ctrl<128 then
     for i=1,ctrl+1 do
      mdat[L][idx]=peek(p)
      p+=1
      idx+=1
     end
    else
     local rep=ctrl-125
     local val=peek(p)
     p+=1
     for i=1,rep do
      mdat[L][idx]=val
      idx+=1
     end
    end
   end
  end
 end

 -- 3. decode entities
 local ne=peek(p)
 p+=1
 for i=1,ne do
  local e={type=peek(p),tx=peek(p+1),
   ty=peek(p+2),group=peek(p+3),
   state=0,anim_t=0}
  add(ents,e)
  p+=4
  if not ent_grp[e.group] then
   ent_grp[e.group]={}
  end
  add(ent_grp[e.group],#ents)
 end

 -- 4. write main tiles (1..nst) to spritesheet
 local nst=lvl_nst
 for t=0,nst-1 do
  local pix=tile_pix[t]
  local tcol=t%8
  local trow=t\8
  for py=0,15 do
   local dst=(trow*16+py)*64+tcol*8
   for px=0,7 do
    local i=py*16+px*2
    local lo=pix[i+1]&0xf
    local hi=pix[i+2]&0xf
    poke(dst+px,lo|(hi<<4))
   end
  end
 end

 -- 4. write bg tiles (nst+1..nt) to user
 -- memory at 0x4300 (128 bytes each, 4bpp)
 -- replace transparent with bg_clr so
 -- memcpy to screen works without
 -- per-pixel transparency checks
 local bg_base=0x8000
 local bg_clr=1 -- matches cls() color
 for t=nst,nt-1 do
  local pix=tile_pix[t]
  local addr=bg_base+(t-nst)*128
  for py=0,15 do
   for px=0,7 do
    local i=py*16+px*2
    local lo=pix[i+1]&0xf
    local hi=pix[i+2]&0xf
    if lo==trans then lo=bg_clr end
    if hi==trans then hi=bg_clr end
    poke(addr+py*8+px,lo|(hi<<4))
   end
  end
 end
end

function tile_at(tx,ty)
 if tx<0 or tx>=lvl_w or ty<0 or ty>=lvl_h then return 0 end
 return mdat[2][ty*lvl_w+tx+1]\4
end

function tile_flag(tx,ty)
 local t=tile_at(tx,ty)
 if t==0 then return 0 end
 return tflg[t] or 0
end

function tile_solid(tx,ty)
 -- flag bit 0 = solid
 if band(tile_flag(tx,ty),1)>0 then
  return true
 end
 return ent_solid(tx,ty)
end

function tile_platform(tx,ty)
 -- flag bit 1 = one-way platform
 return band(tile_flag(tx,ty),2)>0
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

function check_platforms(bx0,bx1,by1)
 -- check one-way platforms at feet row
 -- only land if feet were above last frame
 local tx0=flr(bx0/16)
 local tx1=flr((bx1-0.01)/16)
 local ty=flr((by1-0.01)/16)
 for tx=tx0,tx1 do
  if tile_platform(tx,ty) then
   local ptop=ty*16
   if prev_by1<=ptop then
    return true,ptop
   end
  end
 end
 return false
end

function land_on(y_top)
 py=y_top-cb_y1
 vy=0
 if not grounded then
  grounded=true
  air_time=0
  if state=="fall" then
   set_anim(a_land)
   state="land"
  end
 end
end

function resolve_y()
 local bx0=px+cb_x0
 local by0=py+cb_y0
 local bx1=px+cb_x1
 local by1=py+cb_y1
 local solid=box_hits_solid(bx0,by0,bx1,by1)
 if solid then
  if vy>=0 then
   local ty1=flr((by1-0.01)/16)
   land_on(ty1*16)
  elseif vy<0 then
   local ty0=flr(by0/16)
   py=(ty0+1)*16-cb_y0
   vy=0
  end
  return
 end
 -- no solid: check one-way platforms
 if vy>=0 then
  local hit,ptop=check_platforms(
   bx0,bx1,by1)
  if hit then
   land_on(ptop)
   return
  end
 end
 grounded=false
end

function draw_bg_layer()
 -- bg tiles stored at 0x8000 in user mem
 -- 128 bytes each (16 rows x 8 bytes)
 -- blit to screen via memcpy (no spr())
 local md=mdat[1]
 if not md then return end
 local plx=lplx[1]
 local cx=flr(cam_x*plx)\2*2
 local cy=flr(cam_y*plx)
 local ts=16
 local tx0=max(0,flr(cx/ts))
 local ty0=max(0,flr(cy/ts))
 local tx1=min(lvl_w-1,tx0+8)
 local ty1=min(lvl_h-1,ty0+8)
 local nst=lvl_nst
 for ty=ty0,ty1 do
  for tx=tx0,tx1 do
   local c=md[ty*lvl_w+tx+1]
   if c>0 then
    local sx=tx*ts-cx
    local sy=ty*ts-cy
    if c<=nst then
     local sc=(c-1)%8
     local sr=(c-1)\8
     spr(sr*32+sc*2,sx,sy,2,2)
    else
     local src=0x8000+(c-nst-1)*128
     -- clip x: tile is 8 bytes (16px)
     local x0=max(0,-sx\2)   -- skip bytes on left
     local x1=min(7,(127-sx)\2) -- last byte on right
     if x1>=x0 then
      local w=x1-x0+1
      for py=0,15 do
       local dy=sy+py
       if dy>=0 and dy<128 then
        memcpy(0x6000+dy*64+sx\2+x0,
         src+py*8+x0,w)
       end
      end
     end
    end
   end
  end
 end
end

function draw_main_layer()
 local md=mdat[2]
 if not md then return end
 local plx=lplx[2]
 local cx=cam_x*plx
 local cy=cam_y*plx
 local ts=16
 local tx0=max(0,flr(cx/ts))
 local ty0=max(0,flr(cy/ts))
 local tx1=min(lvl_w-1,tx0+8)
 local ty1=min(lvl_h-1,ty0+8)
 for ty=ty0,ty1 do
  for tx=tx0,tx1 do
   local c=md[ty*lvl_w+tx+1]
   if c>0 then
    local t=c\4
    local fx=band(c,2)>0
    local fy=band(c,1)>0
    local sc=(t-1)%8
    local sr=(t-1)\8
    spr(sr*32+sc*2,tx*ts-cx,ty*ts-cy,
     2,2,fx,fy)
   end
  end
 end
end

function update_camera()
 local target_x=px-64
 local target_y=py-64
 -- clamp to map bounds
 target_x=mid(0,target_x,lvl_w*16-128)
 target_y=mid(0,target_y,lvl_h*16-128)
 -- smooth follow, round to whole pixels
 cam_x+=flr((target_x-cam_x)*0.15+0.5)
 cam_y+=flr((target_y-cam_y)*0.15+0.5)
end

-- -- game --

function _init()
 palt(0,false)
 palt(trans,true)
 cache_anims()
 -- load tiles into sprite sheet (overwrites __gfx__)
 -- entity tiles in __gfx__ survive since they're after map tiles
 load_tiles()
 init_ents()
 -- set player to spawn
 px=spn_x*16+8
 py=spn_y*16
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
 atk_px0=px
 state="attack"
 buf_atk=0
end

function start_sweep()
 vx=0
 set_anim(a_sweep)
 atk_anchor0=anc[a_sweep][1]
 atk_px0=px
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
 atk_px0=px
 state="sweep"
 buf_atk=0
 buf_sweep=0
end

function try_move_x(dx)
 -- move px by dx, stepping to avoid
 -- skipping through thin walls
 local step=cb_x1-cb_x0 -- box width
 local rem=abs(dx)
 local dir=1
 if dx<0 then dir=-1 end
 while rem>0 do
  local d=min(rem,step)
  px+=d*dir
  rem-=d
  local bx0=px+cb_x0
  local by0=py+cb_y0
  local bx1=px+cb_x1
  local by1=py+cb_y1
  if box_hits_solid(bx0,by0,bx1,by1) then
   if dir>0 then
    local tx1=flr((bx1-0.01)/16)
    px=tx1*16-cb_x1
   else
    local tx0=flr(bx0/16)
    px=(tx0+1)*16-cb_x0
   end
   return
  end
 end
end

function apply_atk_drift()
 -- move px each frame to follow anchor
 -- drift, with wall collision
 local ax=anc[cur_anim][cur_frame]
 local drift=ax-atk_anchor0
 local target=atk_px0+drift*facing
 local dx=target-px
 if dx~=0 then try_move_x(dx) end
end

function end_attack()
 local push=atk_push[cur_anim] or 0
 if push~=0 then
  try_move_x(push*facing)
 end
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

function spawn_parts(wx,wy)
 -- spawn 3 bouncing red balls at world pos
 for i=1,3 do
  add(parts,{
   x=wx,y=wy,
   vx=rnd(3)-1.5,
   vy=-rnd(2)-1,
   landed=false,
   age=0
  })
 end
end

function check_atk_tiles()
 -- attack hitbox: extends in facing dir
 local ax0,ax1
 if facing>0 then
  ax0=px+cb_x1
  ax1=ax0+14
 else
  ax1=px+cb_x0
  ax0=ax1-14
 end
 local ay0=py+cb_y0
 local ay1=py+cb_y1
 local tx0=flr(ax0/16)
 local ty0=flr(ay0/16)
 local tx1=flr((ax1-0.01)/16)
 local ty1=flr((ay1-0.01)/16)
 for ty=ty0,ty1 do
  for tx=tx0,tx1 do
   if band(tile_flag(tx,ty),4)>0 then
    -- destroy tile, spawn particles
    local c=mdat[2][ty*lvl_w+tx+1]
    mdat[2][ty*lvl_w+tx+1]=0
    spawn_parts(tx*16+8,ty*16+8)
   end
  end
 end
end

function update_parts()
 for p in all(parts) do
  if not p.landed then
   p.vy+=0.15
   -- move x, check walls
   p.x+=p.vx
   local tx=flr(p.x/16)
   local ty=flr(p.y/16)
   if tile_solid(tx,ty) then
    p.x-=p.vx
    p.vx*=-0.3
   end
   -- move y, check floor/ceiling
   p.y+=p.vy
   tx=flr(p.x/16)
   ty=flr(p.y/16)
   if tile_solid(tx,ty) then
    if p.vy>0 then
     -- hit floor: snap to top
     p.y=ty*16-1
    else
     -- hit ceiling: snap to bottom
     p.y=(ty+1)*16+1
    end
    if abs(p.vy)<0.8 then
     p.landed=true
     p.vx,p.vy=0,0
    else
     p.vy*=-0.4
     p.vx*=0.7
    end
   end
  end
  p.age+=1
  -- collect if player overlaps (after 15 frames)
  local dx=p.x-px
  local dy=p.y-(py+11)
  if p.age>15 and abs(dx)<10 and abs(dy)<10 then
   gems+=1
   del(parts,p)
  end
 end
end

-- -- entity system --

function init_ents()
 for e in all(ents) do
  if e.type==1 then
   -- door: closed = frame 15
   e.anim=a_door e.frame=15
   e.state=0
  elseif e.type==2 then
   -- switch: off = idle frame 1
   e.anim=a_sid e.frame=1
   e.state=0 e.cooldown=0
  end
 end
end

function ent_solid(tx,ty)
 for e in all(ents) do
  if e.type==1
   and (e.state==0 or e.state==3)
  then
   if tx==e.tx
    and ty>=e.ty and ty<e.ty+3 then
    return true
   end
  end
 end
 return false
end

function toggle_switch(e)
 if e.cooldown>0 then return end
 e.cooldown=30
 if e.state==0 then
  -- off -> activating (press down)
  e.state=1 e.anim=a_sdn e.frame=1
 elseif e.state==2 then
  -- on -> deactivating (pop back up)
  e.state=3 e.anim=a_sst e.frame=1
 end
 -- trigger linked doors
 local grp=ent_grp[e.group]
 if grp then
  for _,ei in ipairs(grp) do
   local de=ents[ei]
   if de.type==1 then
    if e.state==1 then
     de.state=1 de.frame=1
    elseif e.state==3 then
     de.state=3 de.frame=14
    end
   end
  end
 end
end

function check_atk_ents()
 local ax0,ax1
 if facing>0 then
  ax0=px+cb_x1 ax1=ax0+14
 else
  ax1=px+cb_x0 ax0=ax1-14
 end
 local ay0,ay1=py+cb_y0,py+cb_y1
 for e in all(ents) do
  if e.type==2 then
   local sx=e.tx*16
   local sy=e.ty*16
   if ax1>sx and ax0<sx+16
    and ay1>sy and ay0<sy+16 then
    toggle_switch(e)
   end
  end
 end
end

function update_ents()
 for e in all(ents) do
  -- switch cooldown
  if e.type==2 and e.cooldown>0 then
   e.cooldown-=1
  end
  -- switch animation
  if e.type==2 then
   if e.state==1 then
    -- activating: play sw_down (press)
    e.anim_t=(e.anim_t or 0)+1
    if e.anim_t>=aspd[a_sdn] then
     e.anim_t=0
     if e.frame<acache[a_sdn].ai.nf then
      e.frame+=1
     else
      -- on: hold last frame of down
      e.state=2 e.frame=acache[a_sdn].ai.nf
     end
    end
   elseif e.state==3 then
    -- deactivating: play sw_start (pop up)
    e.anim_t=(e.anim_t or 0)+1
    if e.anim_t>=aspd[a_sst] then
     e.anim_t=0
     if e.frame<acache[a_sst].ai.nf then
      e.frame+=1
     else
      e.state=0 e.anim=a_sid e.frame=1
     end
    end
   end
  end
  -- door animation
  if e.type==1 then
   if e.state==1 then
    -- opening: play frames 1->14
    e.anim_t=(e.anim_t or 0)+1
    if e.anim_t>=aspd[a_door] then
     e.anim_t=0
     if e.frame<14 then
      e.frame+=1
     else
      e.state=2 e.frame=14
     end
    end
   elseif e.state==3 then
    -- closing: play frames 14->1
    e.anim_t=(e.anim_t or 0)+1
    if e.anim_t>=aspd[a_door] then
     e.anim_t=0
     if e.frame>1 then
      e.frame-=1
     else
      e.state=0 e.frame=15
     end
    end
   end
  end
 end
end

function draw_ent(a,f,sx,sy)
 local buf,bx,by,bw,bh=get_frame(a,f)
 if bw==0 then return end
 local idx=1
 for y=0,bh-1 do
  for x=0,bw-1 do
   local col=buf[idx]
   if col~=trans then
    pset(sx+bx+x,sy+by+y,col)
   end
   idx+=1
  end
 end
end

function draw_ents()
 for e in all(ents) do
  local sx=e.tx*16-cam_x
  local sy=e.ty*16-cam_y
  if e.type==1 then sx-=16 end
  draw_ent(e.anim,e.frame,sx,sy)
 end
end

function _update60()
 if gs==0 then
  if btnp(4) or btnp(5) then gs=1 end
  return
 end
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
  -- walked off ledge (wait 4 frames)
  if not grounded and air_time>4 then
   set_anim(a_fall)
   state="fall"
  -- buffered jump
  elseif buf_jump>0 then
   do_jump()
  -- buffered z: start combo
  elseif buf_atk>0 then
   start_combo()
  -- buffered x: sweep
  elseif buf_sweep>0 then
   start_sweep()
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
  apply_atk_drift()
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
    atk_px0=px
   else
    combo_idx=0
    set_anim(a_idle)
    state="idle"
   end
  end

 elseif state=="sweep" then
  if not air_atk then apply_atk_drift() end
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
     atk_px0=px
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

 end

 -- -- physics --
 prev_by1=py+cb_y1
 if not grounded then
  vy+=grav
  if vy>max_fall then vy=max_fall end
  air_time+=1
 else
  air_time=0
 end

 -- move X, then resolve X collisions
 px+=vx
 resolve_x()

 -- move Y, then resolve Y collisions
 py+=vy
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

 -- attack hitbox vs destructibles + entities
 if state=="attack" or state=="sweep" then
  check_atk_tiles()
  check_atk_ents()
 end

 -- update entities & particles
 update_ents()
 update_parts()

 -- update camera
 update_camera()

 -- tick animation
 tick_anim()
end

function draw_title()
 local buf,bx,by,bw,bh=get_frame(a_title,1)
 if bw==0 then return end
 local idx=1
 for y=0,bh-1 do
  for x=0,bw-1 do
   pset(bx+x,by+y,buf[idx])
   idx+=1
  end
 end
end

function _draw()
 if gs==0 then
  draw_title()
  return
 end
 cls(1)

 -- bg + main layers
 draw_bg_layer()
 draw_main_layer()

 -- draw entities
 draw_ents()

 -- draw player anchored to body center
 local ax=anc[cur_anim][cur_frame]
 local flip=facing==-1
 local vx=px
 local dx
 if flip then
  dx=vx-(cell_w-1-ax)
 else
  dx=vx-ax
 end
 draw_char(cur_anim,cur_frame,dx-cam_x,py-cam_y,flip)

 -- draw particles
 for p in all(parts) do
  circfill(p.x-cam_x,p.y-cam_y,2,8)
  circfill(p.x-cam_x-1,p.y-cam_y-1,1,14)
 end

 -- hud: gem counter
 print(gems,2,2,8)
end
