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

-- fade: v=0 clear, v=8 black, d=direction (1=out,-1=in,0=idle)
fade_v=8 fade_d=-1 fade_t=0
-- screen palette darkening table (colors 1-15, 8 steps toward black)
-- flat: row c = indices (c-1)*8+1..(c-1)*8+8, access via fdp[(c-1)*8+v]
fdp=split"1,1,1,1,0,0,0,0,2,2,2,1,1,0,0,0,3,3,4,5,2,1,1,0,4,4,2,2,1,1,1,0,5,5,2,2,1,1,1,0,6,6,13,5,2,1,1,0,7,7,6,13,5,2,1,0,8,8,9,4,5,2,1,0,9,9,4,5,2,1,1,0,10,15,9,4,5,2,1,0,11,11,3,4,5,2,1,0,12,12,13,5,5,2,1,0,13,13,5,5,2,1,1,0,14,9,9,4,5,2,1,0,15,14,9,4,5,2,1,0"
function apply_fade(v)
 if v==0 then
  for c=0,15 do pal(c,c,1) end
  return
 end
 for c=1,15 do pal(c,fdp[(c-1)*8+v],1) end
 pal(0,0,1)
end

-- anim speeds (frames per anim frame)
-- order: idle,run,jump,fall,hit,land,atk1,xslice,sweep,death,door,sw_start,sw_idle,sw_down,title
aspd=split"6,5,5,5,5,2,6,6,4,6,6,6,6,8,30"

-- physics
grav,max_fall,jump_spd,run_spd,friction=0.15,3,-3.5,1.5,0.8

-- spider surface-crawling tables
-- indexed by surf+1: 1=floor,2=rwall,3=ceil,4=lwall
sf_dx=split"1,0,-1,0"  -- canonical forward dx
sf_dy=split"0,-1,0,1"  -- canonical forward dy
sd_dx=split"0,1,0,-1"  -- surface "down" dx
sd_dy=split"1,0,-1,0"  -- surface "down" dy
sp_rot=split"0,3,2,1"  -- draw rotation per surface
snpx=split"8,16,8,-1"  -- snap probe x offset per surface
snpy=split"16,8,-1,8"  -- snap probe y offset per surface

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

-- player HP
plr_hp=3
plr_inv=0

-- spider projectiles
sprojs={}
-- spider anim speeds: idle,walk,attack,hit,death
sp_aspd=split"8,6,5,5,6"
-- wb anim speeds: idle,move,charge,shoot,fire_dash,wake,damaged,death
wb_aspd=split"8,5,6,5,4,6,5,6"

-- entity system
ents={}
ent_grp={}

-- -- decoders --

function pk2(a)
 return peek(a)+peek(a+1)*256
end

function decode_eg2(off,npix,bpp,w)
 local by=peek(off) off+=1
 local bi=0
 local function rb()
  local v=(by>>bi)&1 bi+=1
  if bi>7 then by=peek(off) off+=1 bi=0 end
  return v
 end
 local mode=rb()+rb()*2+rb()*4
 local k=rb()+rb()*2+1
 local ks=1<<k
 local function eg()
  local z=0
  while rb()==0 do z+=1 end
  local v=1
  for i=1,z+k do v=v*2+rb() end
  return v-ks
 end
 local buf={}
 local pb,pv,pi=0,0,1
 while pi<=npix do
  local z=eg()
  for i=1,z do
   pv=pv*2 pb+=1
   if pb==bpp then buf[pi]=pv pi+=1 pv=0 pb=0 end
  end
  if pi<=npix then
   pv=pv*2+1 pb+=1
   if pb==bpp then buf[pi]=pv pi+=1 pv=0 pb=0 end
  end
 end
 if mode==1 then
  for i=2,npix do
   if (i-1)%w>0 then buf[i]=buf[i]^^buf[i-1] end
  end
 elseif mode==2 then
  for i=w+1,npix do buf[i]=buf[i]^^buf[i-w] end
 elseif mode==3 then
  for i=w+2,npix do
   if (i-1)%w>0 then buf[i]=buf[i]^^buf[i-w-1] end
  end
 elseif mode==4 then
  for i=2,npix do
   local x=(i-1)%w
   local a,b,c=x>0 and buf[i-1] or 0,i>w and buf[i-w] or 0,x>0 and i>w and buf[i-w-1] or 0
   local p,r=a+b-c
   local pa,pc=abs(p-a),abs(p-c)
   r=pa<=abs(p-b) and pa<=pc and a or abs(p-b)<=pc and b or c
   buf[i]=buf[i]^^r
  end
 end
 return buf
end

function read_anim(a,cb)
 cb=cb or char_base
 local na=peek(cb)
 local ab=cb+3+na*2+pk2(cb+3+(a-1)*2)
 local nf=peek(ab)
 local bpp=peek(ab+2)
 local np=bpp<4 and (1<<bpp) or 0
 local pal={}
 for i=0,np-1 do
  local b=peek(ab+3+i\2)
  pal[i+1]=i%2==0 and (b>>4)&0xf or b&0xf
 end
 local h=ab+3+np\2
 local bx,by,bw,bh=peek(h,4)
 local ro=h+4
 return {
  nf=nf,bpp=bpp,pal=pal,
  bx=bx,by=by,bw=bw,bh=bh,
  ref_off=ro,fo_off=ro+nf,
  data_off=ro+nf*3
 }
end

acache={}
sp_anc={}

function decode_anim(ai)
 local frames={}
 local npix=ai.bw*ai.bh
 local zeros={}
 for i=1,npix do zeros[i]=0 end
 for f=1,ai.nf do
  local ref=peek(ai.ref_off+f-1)
  local foff=pk2(ai.fo_off+(f-1)*2)
  local d=decode_eg2(ai.data_off+foff,npix,ai.bpp,ai.bw)
  local base=ref==255 and zeros or frames[ref+1][1]
  for i=1,npix do d[i]=d[i]^^base[i] end
  frames[f]={d,ai.bx,ai.by,ai.bw,ai.bh}
 end
 if ai.bpp<4 and #ai.pal>0 then
  for f=1,#frames do
   local buf=frames[f][1]
   for i=1,#buf do
    buf[i]=ai.pal[buf[i]+1] or trans
   end
  end
 end
 return frames
end

function cache_anims()
 local na=peek(char_base)
 for a=1,na do
  local ai=read_anim(a,char_base)
  acache[a]={ai=ai,frames=decode_anim(ai),cw=cell_w,ch=cell_h}
 end
 local ai=read_anim(1,title_base)
 acache[a_title]={ai=ai,frames=decode_anim(ai),cw=128,ch=128}
 ai=read_anim(1,font_base)
 acache[a_font]={ai=ai,frames=decode_anim(ai),cw=font_cw,ch=font_ch}
 local sna=peek(spider_base)
 for a=1,sna do
  ai=read_anim(a,spider_base)
  local idx=a_spi+a-1
  acache[idx]={ai=ai,frames=decode_anim(ai),cw=spider_cw,ch=spider_ch}
 end
 local wna=peek(wheelbot_base)
 for a=1,wna do
  ai=read_anim(a,wheelbot_base)
  local idx=a_wbi+a-1
  acache[idx]={ai=ai,frames=decode_anim(ai),cw=wheelbot_cw,ch=wheelbot_ch}
 end
 -- hp bar
 ai=read_anim(1,hp_base)
 hp_buf=decode_anim(ai)[1][1]
end

function hurt_plr()
 if plr_inv==0 then
  plr_hp-=1
  plr_inv=60
 end
end

function draw_hp()
 local fill=hp_w*(1-plr_hp/3)
 local tt=time()*3
 local idx=1
 for y=0,hp_h-1 do
  local wave=sin(y*0.12+tt)*1.5
  for x=0,hp_w-1 do
   local c=hp_buf[idx]
   if c==0 then
    pset(2+x,2+y,0)
   elseif c==7 then
    if hp_w-1-x>=fill+wave then
     pset(2+x,2+y,8)
    end
   end
   idx+=1
  end
 end
end

function get_frame(a,f)
 local fr=acache[a].frames[f]
 return fr[1],fr[2],fr[3],fr[4],fr[5]
end

function draw_char(a,f,sx,sy,flip,rot)
 local buf,bx,by,bw,bh=get_frame(a,f)
 if bw==0 then return end
 local cw=acache[a].cw
 local idx=1
 for y=0,bh-1 do
  for x=0,bw-1 do
   local col=buf[idx]
   if col~=trans then
    local dx,dy
    if flip then
     dx=cw-1-bx-x
    else
     dx=bx+x
    end
    dy=by+y
    if rot==1 then
     dx,dy=cw-1-dy,dx
    elseif rot==2 then
     dx,dy=cw-1-dx,cw-1-dy
    elseif rot==3 then
     dx,dy=dy,cw-1-dx
    end
    pset(sx+dx,sy+dy,col)
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
 local nt,nl=peek(map_base,2)
 local p=map_base+12+nl
 local tbpp=peek(p)
 local tnp=1<<tbpp
 local tpal={}
 for i=0,tnp-1 do
  local tb=peek(p+1+i\2)
  tpal[i]=i%2==0 and (tb>>4)&0xf or tb&0xf
 end
 local all_idx=decode_eg2(p+1+tnp\2,nt*256,tbpp,16)
 p+=pk2(map_base+10)
 for L=1,nl do
  mdat[L]={}
  for i=1,sz do mdat[L][i]=0 end
  local m=peek(map_base+11+L)
  if m==1 then
   local tw,th,dx,dy,rw,rh=peek(p,6)
   for ry=0,rh-1 do
    for rx=0,rw-1 do
     mdat[L][(dy+ry)*lvl_w+dx+rx+1]=peek(p+6+(ry%th)*tw+(rx%tw))
    end
   end
   p+=6+tw*th
  elseif m==2 then
   local idx=1
   while idx<=sz do
    local ctrl=peek(p) p+=1
    if ctrl<128 then
     for i=1,ctrl+1 do
      mdat[L][idx]=peek(p) p+=1 idx+=1
     end
    else
     local val=peek(p) p+=1
     for i=1,ctrl-125 do
      mdat[L][idx]=val idx+=1
     end
    end
   end
  end
 end
 local ne=peek(p) p+=1
 for i=1,ne do
  local a,b,c,d=peek(p,4)
  local e={type=a,tx=b,ty=c,group=d}
  add(ents,e) p+=4
  ent_grp[d]=ent_grp[d] or {}
  add(ent_grp[d],#ents)
 end
 for t=0,nt-1 do
  local bg=t>=lvl_nst
  for py=0,15 do
   local dst
   if bg then
    dst=0x8000+(t-lvl_nst)*128+py*8
   else
    dst=(t\8*16+py)*64+t%8*8
   end
   for px=0,7 do
    local i=t*256+py*16+px*2
    local lo=tpal[all_idx[i+1]]&0xf
    local hi=tpal[all_idx[i+2]]&0xf
    if bg then
     if lo==trans then lo=1 end
     if hi==trans then hi=1 end
    end
    poke(dst+px,lo|(hi<<4))
   end
  end
 end
 all_idx=nil
end

function thi(v) return flr((v-0.01)/16) end

function tile_flag(tx,ty)
 if tx<0 or tx>=lvl_w or ty<0 or ty>=lvl_h then return 0 end
 local t=mdat[2][ty*lvl_w+tx+1]\4
 if t==0 then return 0 end
 return tflg[t] or 0
end

function tile_solid(tx,ty)
 if tile_flag(tx,ty)&1>0 then
  return true
 end
 return ent_solid(tx,ty)
end

function box_hits_solid(bx0,by0,bx1,by1)
 local tx0=flr(bx0/16)
 local ty0=flr(by0/16)
 local tx1=thi(bx1)
 local ty1=thi(by1)
 for ty=ty0,ty1 do
  for tx=tx0,tx1 do
   if tile_solid(tx,ty) then
    return true
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
 if box_hits_solid(bx0,by0,bx1,by1) then
  if vy>=0 then
   land_on(thi(by1)*16)
  elseif vy<0 then
   py=(flr(by0/16)+1)*16-cb_y0
   vy=0
  end
  return
 end
 if vy>=0 then
  local tx0=flr(bx0/16)
  local tx1=thi(bx1)
  local ty=thi(by1)
  local ptop=ty*16
  if prev_by1<=ptop then
   for tx=tx0,tx1 do
    if tile_flag(tx,ty)&2>0 then
     land_on(ptop)
     return
    end
   end
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
    local fx=c&2>0
    local fy=c&1>0
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
    px=thi(bx1)*16-cb_x1
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

function atk_box()
 local ax0,ax1
 if facing>0 then
  ax0=px+cb_x1 ax1=ax0+14
 else
  ax1=px+cb_x0 ax0=ax1-14
 end
 return ax0,py+cb_y0,ax1,py+cb_y1
end

function check_atk_tiles()
 local ax0,ay0,ax1,ay1=atk_box()
 local tx0=flr(ax0/16)
 local ty0=flr(ay0/16)
 local tx1=thi(ax1)
 local ty1=thi(ay1)
 for ty=ty0,ty1 do
  for tx=tx0,tx1 do
   if tile_flag(tx,ty)&4>0 then
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
  elseif e.type==3 then
   -- spider
   e.x=e.tx*16 e.y=e.ty*16
   e.surf=0 e.mdir=1
   e.anim=a_spi e.frame=1
   e.state="idle"
   e.anim_t=0 e.hp=3
   e.atk_cd=0 e.fired=false
   e.inv_t=0 e.move_acc=0
   e.patrol_t=60 e.walk_count=0
  elseif e.type==4 then
   e.x=e.tx*16+8 e.y=(e.ty+1)*16
   e.vx=0 e.vy=0 e.mdir=1
   e.anim=a_wbi e.frame=1
   e.state="sleep" e.anim_t=0
   e.hp=4 e.inv_t=0 e.atk_cd=0
   e.grounded=false e.fired=false
   e.patrol_t=0 e.walk_count=0
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
 local ax0,ay0,ax1,ay1=atk_box()
 for e in all(ents) do
  if e.type==2 then
   local sx=e.tx*16
   local sy=e.ty*16
   if ax1>sx and ax0<sx+16
    and ay1>sy and ay0<sy+16 then
    toggle_switch(e)
   end
  elseif e.type==3 and e.hp>0
   and e.state~="death" and e.inv_t==0 then
   if ax1>e.x and ax0<e.x+16
    and ay1>e.y and ay0<e.y+16 then
    ent_hurt(e,a_spd,a_sph)
   end
  elseif e.type==4 and e.hp>0
   and e.state~="death" and e.state~="sleep"
   and e.state~="fdash" and e.inv_t==0 then
   if ax1>e.x-14 and ax0<e.x+14
    and ay1>e.y-24 and ay0<e.y then
    ent_hurt(e,a_wbdt,a_wbd)
   end
  end
 end
end

function ent_hurt(e,da,ha)
 e.hp-=1 e.inv_t=20
 if e.hp<=0 then
  e.state="death" e.vx=0
  sp_set_anim(e,da)
 else
  e.state="hit" e.vx=0
  sp_set_anim(e,ha)
 end
end

function fire_at(sx,sy,spd)
 local dx,dy=px-sx,py+11-sy
 local d=max(abs(dx),abs(dy))
 if d>0 then
  add(sprojs,{x=sx,y=sy,dx=dx/d*spd,dy=dy/d*spd,exp=false,et=0})
 end
end

function sp_set_anim(e,a)
 if e.anim~=a then
  e.anim=a e.frame=1 e.anim_t=0
 end
end

function go_idle(e,a,pt)
 e.state="idle" sp_set_anim(e,a)
 e.patrol_t=pt
end

function patrol_start(e,a)
 e.walk_count+=1
 if e.walk_count%2==0 then e.mdir=-e.mdir end
 e.patrol_t=120
 sp_set_anim(e,a)
end

-- probe: check two tile_solid points
-- along an edge in direction dx,dy
function sp_probe(x,y,dx,dy)
 if dx~=0 then
  local ex=dx>0 and x+16 or x-1
  return tile_solid(flr(ex/16),flr((y+2)/16))
   or tile_solid(flr(ex/16),flr((y+13)/16))
 else
  local ey=dy>0 and y+16 or y-1
  return tile_solid(flr((x+2)/16),flr(ey/16))
   or tile_solid(flr((x+13)/16),flr(ey/16))
 end
end

-- snap spider feet to nearest
-- solid tile on current surface
function sp_snap(e)
 local s=e.surf+1
 local dx,dy=sd_dx[s],sd_dy[s]
 local tx,ty=flr((e.x+snpx[s])/16),flr((e.y+snpy[s])/16)
 if tile_solid(tx,ty) then
  if dx~=0 then e.x=(tx-sgn(dx))*16
  else e.y=(ty-sgn(dy))*16 end
 end
end

-- move spider 1px along surface
function step_spider(e)
 local s=e.surf+1
 local fdx=sf_dx[s]*e.mdir
 local fdy=sf_dy[s]*e.mdir
 local nx=e.x+fdx
 local ny=e.y+fdy

 if sp_probe(nx,ny,fdx,fdy) then
  -- inner corner: wall ahead
  e.surf=(e.surf+e.mdir)%4
  sp_snap(e)
 elseif not sp_probe(nx,ny,sd_dx[s],sd_dy[s]) then
  -- outer corner: no ground ahead
  -- shift position into the surface
  e.x+=sd_dx[s]*16
  e.y+=sd_dy[s]*16
  e.surf=(e.surf-e.mdir+4)%4
  sp_snap(e)
 else
  -- straight movement
  e.x=nx
  e.y=ny
 end
end

function wb_move(e)
 if not e.grounded then
  e.vy+=0.15
  if e.vy>3 then e.vy=3 end
 end
 e.x+=e.vx
 if box_hits_solid(e.x-14,e.y-24,e.x+14,e.y) then
  if e.vx>0 then e.x=flr((e.x+13.99)/16)*16-14
  elseif e.vx<0 then e.x=flr((e.x-14)/16)*16+16+14 end
  e.vx=0
 end
 e.y+=e.vy
 if box_hits_solid(e.x-14,e.y-24,e.x+14,e.y) then
  if e.vy>=0 then
   e.y=thi(e.y)*16
   e.vy=0 e.grounded=true
  else
   e.y=flr((e.y-24)/16)*16+16+24
   e.vy=0
  end
 else e.grounded=false end
end

function ent_tick(e,spd)
 e.anim_t=(e.anim_t or 0)+1
 if e.anim_t>=spd then
  e.anim_t=0
  return true,acache[e.anim].ai.nf
 end
 return false,0
end

function update_spider(e)
 local spd=sp_aspd[e.anim-a_spi+1] or 6
 -- death: tick anim only, no movement
 if e.state=="death" then
  local tick,nf=ent_tick(e,spd)
  if tick and e.frame<nf then
   e.frame+=1
  end
  return
 end

 if e.inv_t>0 then e.inv_t-=1 end
 if e.atk_cd>0 then e.atk_cd-=1 end

 -- tick animation
 local tick,nf=ent_tick(e,spd)
 if tick then
  if e.state=="hit" then
   if e.frame<nf then e.frame+=1
   else
    go_idle(e,a_spi,60)
   end
  elseif e.state=="attack" then
   if e.frame==3 and not e.fired then
    e.fired=true
    fire_at(e.x+8,e.y+8,2)
   end
   if e.frame<nf then e.frame+=1
   else
    e.atk_cd=180
    e.fired=false
    go_idle(e,a_spi,60)
   end
  elseif e.state=="walk" then
   e.frame=e.frame%nf+1
  end
 end

 -- spot player
 if (e.state=="idle" or e.state=="walk") and e.atk_cd==0 then
  local ddx=px-e.x
  local ddy=py-e.y
  if abs(ddx)<80 and abs(ddy)<48 then
   -- face player along surface
   local s=e.surf+1
   local fdx=sf_dx[s]
   local fdy=sf_dy[s]
   local dot=ddx*fdx+ddy*fdy
   e.mdir=dot>=0 and 1 or -1
   e.state="attack" sp_set_anim(e,a_spa)
   return
  end
 end

 -- patrol: idle <-> walk
 if e.state=="walk" then
  e.move_acc+=0.5
  while e.move_acc>=1 do
   e.move_acc-=1
   step_spider(e)
  end
 end
 e.patrol_t-=1
 if e.state=="idle" then
  if e.patrol_t<=0 then
   e.state="walk" patrol_start(e,a_spw)
  end
 elseif e.state=="walk" then
  if e.patrol_t<=0 then
   go_idle(e,a_spi,60)
  end
 end
end

function update_wheelbot(e)
 local spd=wb_aspd[e.anim-a_wbi+1] or 6
 if e.state=="death" then
  local t,nf=ent_tick(e,spd)
  if t and e.frame<nf then e.frame+=1 end
  wb_move(e) return
 end
 if e.inv_t>0 then e.inv_t-=1 end
 if e.atk_cd>0 then e.atk_cd-=1 end
 local t,nf=ent_tick(e,spd)
 local ddx=px-e.x
 local ddy=(py+11)-e.y
 local dist=abs(ddx)
 local function spot()
  if e.atk_cd==0 and dist<80 and abs(ddy)<32 then
   e.vx=0 e.mdir=ddx>0 and 1 or -1
   e.state="charge" sp_set_anim(e,a_wbc)
   return true
  end
 end
 if e.state=="sleep" then
  if dist<64 and abs(ddy)<48 then
   e.state="wake" sp_set_anim(e,a_wbwk)
  end
 elseif e.state=="wake" then
  if t then
   if e.frame<nf then e.frame+=1
   else
    go_idle(e,a_wbi,30)
   end
  end
 elseif e.state=="idle" then
  if not spot() then
   e.patrol_t-=1
   if e.patrol_t<=0 then
    e.state="move" patrol_start(e,a_wbm)
   end
  end
 elseif e.state=="move" then
  e.vx=e.mdir*0.6
  if t then e.frame=e.frame%nf+1 end
  if not spot() then
   e.patrol_t-=1
   if e.patrol_t<=0 then
    e.vx=0 go_idle(e,a_wbi,60)
   end
  end
 elseif e.state=="charge" then
  if t then
   if e.frame<nf then e.frame+=1
   else
    if dist>48 then
     e.state="shoot" sp_set_anim(e,a_wbs)
     e.fired=false
    else
     e.state="fdash" sp_set_anim(e,a_wbfd)
     e.inv_t=30
    end
   end
  end
 elseif e.state=="shoot" then
  if t then
   if e.frame==3 and not e.fired then
    e.fired=true
    fire_at(e.x,e.y-12,2.5)
   end
   if e.frame<nf then e.frame+=1
   else
    e.atk_cd=120 go_idle(e,a_wbi,60)
   end
  end
 elseif e.state=="fdash" then
  e.vx=e.mdir*3
  if abs(px-e.x)<18 and abs(py+11-e.y+12)<16 then
   hurt_plr()
  end
  if t then
   if e.frame<nf then e.frame+=1
   else
    e.vx=0 e.atk_cd=120 go_idle(e,a_wbi,60)
   end
  end
 elseif e.state=="hit" then
  if t then
   if e.frame<nf then e.frame+=1
   else
    go_idle(e,a_wbi,60)
   end
  end
 end
 wb_move(e)
end

function update_sprojs()
 for p in all(sprojs) do
  if p.exp then
   p.et+=1
   if p.et>12 then del(sprojs,p) end
  else
   p.x+=p.dx
   p.y+=p.dy
   p.dy+=0.2
   -- wall collision
   if box_hits_solid(p.x-2,p.y-2,p.x+2,p.y+2) then
    p.exp=true p.et=0
   -- player collision
   elseif p.x>px+cb_x0 and p.x<px+cb_x1
    and p.y>py+cb_y0 and p.y<py+cb_y1 then
    p.exp=true p.et=0
    hurt_plr()
   -- off screen
   elseif p.x<cam_x-8 or p.x>cam_x+136
    or p.y<cam_y-8 or p.y>cam_y+136 then
    del(sprojs,p)
   end
  end
 end
end

function update_ents()
 for e in all(ents) do
  if e.type==2 then
   if e.cooldown>0 then e.cooldown-=1 end
   if e.state==1 then
    local tick,nf=ent_tick(e,aspd[a_sdn])
    if tick then
     if e.frame<nf then e.frame+=1
     else e.state=2 e.frame=nf end
    end
   elseif e.state==3 then
    local tick,nf=ent_tick(e,aspd[a_sst])
    if tick then
     if e.frame<nf then e.frame+=1
     else e.state=0 e.anim=a_sid e.frame=1 end
    end
   end
  elseif e.type==1 then
   if e.state==1 then
    if ent_tick(e,aspd[a_door]) then
     if e.frame<14 then e.frame+=1
     else e.state=2 e.frame=14 end
    end
   elseif e.state==3 then
    if ent_tick(e,aspd[a_door]) then
     if e.frame>1 then e.frame-=1
     else e.state=0 e.frame=15 end
    end
   end
  elseif e.type==3 then
   update_spider(e)
  elseif e.type==4 then
   update_wheelbot(e)
  end
 end
end

function p8print(str,x,y,col,sp)
 col=col or 7
 sp=sp or 1
 local cx=x
 for i=1,#str do
  local f=font_map[ord(sub(str,i,i))]
  if f then
   local buf,bx,by,bw,bh=get_frame(a_font,f)
   if bw>0 then
    local idx=1
    for dy=0,bh-1 do
     for dx=0,bw-1 do
      if buf[idx]~=trans then
       pset(cx+bx+dx,y+by+dy,col)
      end
      idx+=1
     end
    end
   end
  end
  cx+=font_cw+sp
 end
end

function draw_ents()
 for e in all(ents) do
  if e.type==4 then
   if not(e.inv_t>0 and e.inv_t%4<2) then
    local a,f=e.anim,e.frame
    local ax=(wb_anc[a] and wb_anc[a][f]) or wheelbot_cw\2
    local flip=e.mdir==-1
    local dx=flip and e.x-(wheelbot_cw-1-ax) or e.x-ax
    draw_char(a,f,dx-cam_x,e.y-wheelbot_ch-cam_y,flip)
   end
  elseif e.type==3 then
   local sx=e.x-cam_x
   local sy=e.y-cam_y
   local flip=e.mdir==-1
   local rot=sp_rot[e.surf+1]
   draw_char(e.anim,e.frame,sx,sy,flip,rot)
   -- antenna blink during idle
   if e.anim==a_spi and (e.anim_t\4)%2==0 then
    local bx=flip and 7 or 8
    local bpx,bpy=bx,12
    if rot==1 then
     bpx,bpy=spider_cw-1-bpy,bx
    elseif rot==2 then
     bpx,bpy=spider_cw-1-bx,spider_cw-1-bpy
    elseif rot==3 then
     bpx,bpy=bpy,spider_cw-1-bx
    end
    pset(sx+bpx,sy+bpy,8)
   end
  else
   local sx=e.tx*16-cam_x
   local sy=e.ty*16-cam_y
   if e.type==1 then sx-=16 end
   draw_char(e.anim,e.frame,sx,sy)
  end
 end
end

function draw_sprojs()
 for p in all(sprojs) do
  local px,py=flr(p.x)-cam_x,flr(p.y)-cam_y
  if p.exp then
   local r=p.et*2
   circ(px,py,r,7)
   if r>3 then circ(px,py,r-3,10) end
   if r>6 then circfill(px,py,r-6,9) end
  else
   circfill(px,py,2,10)
   circfill(px,py,1,7)
  end
 end
end

function _update60()
 -- fade update
 if fade_d~=0 then
  fade_t+=1
  if fade_t>=4 then
   fade_t=0
   fade_v+=fade_d
   if fade_v>=8 then
    fade_v=8
    if gs==0 then gs=1 end
    fade_d=-1
   elseif fade_v<=0 then
    fade_v=0 fade_d=0
   end
  end
 end
 if gs==0 then
  if fade_d==0 and (btnp(4) or btnp(5)) then
   fade_d=1
  end
  return
 end
 if plr_inv>0 then plr_inv-=1 end
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

 -- move X with wall collision
 try_move_x(vx)

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
 update_sprojs()
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
 -- title text (top, red, tight spacing)
 local t1="Ashen Edge"
 local tw=#t1*font_cw+(#t1-1)*-3
 p8print(t1,64-tw\2,10,8,-3)
 -- blinking prompt
 if (time()*2)%2<1 then
  local t2="Press X"
  local pw=#t2*font_cw+(#t2-1)*-3
  p8print(t2,64-pw\2,108,6,-3)
 end
end

function _draw()
 if gs==0 then
  draw_title()
  apply_fade(fade_v)
  return
 end
 cls(1)

 -- bg + main layers
 draw_bg_layer()
 draw_main_layer()

 -- draw entities and projectiles
 draw_ents()
 draw_sprojs()

 -- draw player anchored to body center
 local acw=acache[cur_anim].cw
 local ax=(sp_anc[cur_anim] and sp_anc[cur_anim][cur_frame])
       or (anc[cur_anim] and anc[cur_anim][cur_frame])
       or acw\2
 local flip=facing==-1
 local vx=px
 local dx
 if flip then
  dx=vx-(acw-1-ax)
 else
  dx=vx-ax
 end
 if plr_inv==0 or plr_inv%4<2 then
  draw_char(cur_anim,cur_frame,dx-cam_x,py-cam_y,flip)
 end

 -- draw particles
 for p in all(parts) do
  circfill(p.x-cam_x,p.y-cam_y,2,8)
  circfill(p.x-cam_x-1,p.y-cam_y-1,1,14)
 end

 -- hud
 draw_hp()
 print(gems,2,hp_h+4,8)
 apply_fade(fade_v)
end
