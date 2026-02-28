pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
--##generated##
trans=14
a_spi,a_spw,a_spa,a_sph,a_spd=1,2,3,4,5
spider_base=17152 spider_cw=16 spider_ch=16
spider_data="\005\016\016\000\000\029\000\176\000x\001\189\001\001\001\002\224`\000\000\000\010\009\006\002D\002@\004@\001@\004\128\000D\003C\128\002F\001\006\000\002\224\022\002\000\008\009\008\002\004\027\000\020\000\001\000\000\001\001\001/\000;\000I\000J\000W\000X\000\002D\002@\004@\001@\004\192\000D\003C\192\003C\192\002F\001@\002@\000@\001\011E\001@\004\192\001@\006D\003C\192\003D\004@\001@\002\008\017\000\000\129\131\241\003R!1\000\009\006\000\000\000q\001\000\129\131\241\000\240\002\000\009\017\000\003\128\241\003\129Q@\001\016\001\000\022\012\000\000\000\000\000\000\000 \001\001\001 a\001 \016\000\000!Q#SA\005\000\002\224\022\003\000\000\013\016\000\001\003\029\000\"\000/\000\000\001\002\002\002n\000o\000p\000\142\000\143\000?8D\193\004@\003\195\003@\003\195\002D\001\193\003C\192\006E\128\005@\000@\000@\007?+B\001\194\003@\002A\195\002@\002\198\000D\198\000C\199\000C\192@\196\000F\000\194\001@\002@\007\006\194\008\196\006\198\004\200\003@\199\002@\200\002@\000\198\002@\002\196\003@\003\194\004@\011@\010D\007C\192\007C\192\006F\005@\002@\000@\005\000\000\025\010\000\003\211\211@\240\005@p\243\003Ss3\003s\003\003\003\003\131\003\003\003\003\147\003\003\000\020\007\000\000\000\000\192\000\192\000\192\000\176\000\176\000\160\000\160\000\144\000\000\002\001\002\224`\000\000\031\000\000\008\009\008\002D\002@\004@\001@\004\128\000D\003C\128\003C\128\002F\001@\002@\000@\001\000\008\009\008\002\132\002\128\004\128\001\128\004\128\000\132\003\132\003\132\002\134\001\128\002\128\000\128\001\006\001\002\224`\000\000\029\0005\000N\000\\\000f\000\000\004\009\007\002B\004@\002A\002@\004@\000C\128\001\128\000C\128\002F\002@\001@\003\000\004\009\006\007\128\001C\001@\000C\128A\001C\128\002F\002@\001@\003\000\006\010\008\004@\002\128\002C\000@\002E\003D\003E\003E\002F\003D\003\001\010\006\006\003A\002@\004@\002C\000K\001\011\006\004\002@\002C\000K\001\013\006\003\000C\000K"
_sa=split("4|4,4,4,4,4,4|5,6,6,6,5|4,4|4,4,4,3,3,3","|",false)
sp_anc={} for i=1,#_sa do sp_anc[a_spi+i-1]=split(_sa[i]) end
--##end##

function pk2(a) return peek(a)|(peek(a+1)<<8) end

acache={}

function decode_rle(off,npix,bpp)
 bpp=bpp or 4
 local run_bits=8-bpp
 local run_mask=(1<<run_bits)-1
 local color_mask=(1<<bpp)-1
 local buf={}
 local idx=1
 while idx<=npix do
  local b=peek(off)
  off+=1
  local color=(b>>run_bits)&color_mask
  local r=b&run_mask
  if r==run_mask then
   r=run_mask+1+peek(off)
   off+=1
  else
   r=r+1
  end
  for i=0,r-1 do buf[idx+i]=color end
  idx+=r
 end
 return buf,off
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

function read_anim(a,cb)
 local na=peek(cb)
 local aoff=pk2(cb+3+(a-1)*2)
 local ab=cb+3+na*2+aoff
 local nf=peek(ab)
 local enc=peek(ab+1)
 local bpp=peek(ab+2)
 local np=bpp<4 and (1<<bpp) or 0
 local pal={}
 for i=0,np-1 do
  local b=peek(ab+3+flr(i/2))
  pal[i+1]=(i%2==0) and ((b>>4)&0xf) or (b&0xf)
 end
 local pal_bytes=flr((np+1)/2)
 local h=ab+3+pal_bytes
 if enc==0 then
  local nk=peek(h)
  local bx=peek(h+1)
  local by=peek(h+2)
  local bw=peek(h+3)
  local bh=peek(h+4)
  local ki_off=h+5
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
   bpp=bpp,pal=pal,
   bx=bx,by=by,bw=bw,bh=bh,
   ki_off=ki_off,ks_off=ks_off,
   as_off=as_off,do_off=do_off,
   data_off=data_off,ksz=ksz
  }
 else
  local fo_off=h
  local data_off=fo_off+nf*2
  return {
   enc=1,nf=nf,
   bpp=bpp,pal=pal,
   fo_off=fo_off,data_off=data_off
  }
 end
end

function decode_anim(ai)
 local frames={}
 if ai.enc==0 then
  local npix=ai.bw*ai.bh
  local kbufs={}
  local koff=ai.data_off
  for i=0,ai.nk-1 do
   kbufs[i]=decode_rle(koff,npix,ai.bpp)
   koff+=ai.ksz[i]
  end
  for f=1,ai.nf do
   local ki=peek(ai.as_off+f-1)
   local buf={}
   local kb=kbufs[ki]
   for i=1,#kb do buf[i]=kb[i] end
   local doff=pk2(ai.do_off+(f-1)*2)
   decode_skip(buf,ai.data_off+doff)
   frames[f]={buf,ai.bx,ai.by,ai.bw,ai.bh}
  end
 else
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
    local buf=decode_rle(addr+4,bw*bh,ai.bpp)
    frames[f]={buf,bx,by,bw,bh}
   end
  end
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

function get_frame(a,f)
 local fr=acache[a].frames[f]
 return fr[1],fr[2],fr[3],fr[4],fr[5]
end

function draw_char(a,f,sx,sy,flip)
 local buf,bx,by,bw,bh=get_frame(a,f)
 if bw==0 then return end
 local acw=acache[a].cw
 local idx=1
 for y=0,bh-1 do
  for x=0,bw-1 do
   local col=buf[idx]
   if col~=trans then
    local dx
    if flip then
     dx=acw-1-bx-x
    else
     dx=bx+x
    end
    pset(sx+dx,sy+by+y,col)
   end
   idx+=1
  end
 end
end

-- body anim names (indices 1..5, sp_proj=6 not shown)
anim_names={"idle","walk","attack","hit","death"}
cur_anim=1
cur_frame=1
frame_timer=0
frame_spd=8
do_flip=false

-- spider screen position
spx=56
spy=88
floor_y=108

-- projectile state
projs={}
proj_vx=2
proj_vy=-4
proj_grav=0.2
fired=false

function _init()
 palt(0,false)
 palt(trans,true)
 local p=spider_base
 for i=1,#spider_data do poke(p,ord(spider_data,i)) p+=1 end
 local sna=peek(spider_base)
 for a=1,sna do
  local ai=read_anim(a,spider_base)
  acache[a]={ai=ai,frames=decode_anim(ai),cw=spider_cw,ch=spider_ch}
 end
end

function _update()
 local prev_frame=cur_frame
 if btnp(0) then
  cur_anim-=1
  if cur_anim<1 then cur_anim=#anim_names end
  cur_frame=1
  frame_timer=0
  fired=false
 end
 if btnp(1) then
  cur_anim+=1
  if cur_anim>#anim_names then cur_anim=1 end
  cur_frame=1
  frame_timer=0
  fired=false
 end
 if btnp(4) then
  do_flip=not do_flip
 end
 frame_timer+=1
 if frame_timer>=frame_spd then
  frame_timer=0
  local nf=acache[cur_anim].ai.nf
  cur_frame=cur_frame%nf+1
 end

 -- spawn projectile on attack frame 4 (first true attack frame)
 if cur_anim==a_spa and cur_frame==4 and prev_frame~=4 and not fired then
  fired=true
  local dir=do_flip and -1 or 1
  local sx=spx+4+dir*4
  local sy=spy
  add(projs,{x=sx,y=sy,dx=dir*proj_vx,dy=proj_vy,exp=false,et=0})
 end
 if cur_anim~=a_spa then fired=false end

 -- update projectiles
 for p in all(projs) do
  if p.exp then
   p.et+=1
   if p.et>12 then del(projs,p) end
  else
   p.x+=p.dx
   p.y+=p.dy
   p.dy+=proj_grav
   if p.x<0 or p.x>127 or p.y<0 or p.y>floor_y then
    p.exp=true p.et=0
   end
  end
 end
end

function _draw()
 cls(5)
 -- floor
 rectfill(0,floor_y,127,127,1)
 line(0,floor_y-1,127,floor_y-1,13)

 -- spider
 local ax=(sp_anc[cur_anim] and sp_anc[cur_anim][cur_frame]) or spider_cw\2
 local dx
 if do_flip then
  dx=spx-(spider_cw-1-ax)
 else
  dx=spx-ax
 end
 draw_char(cur_anim,cur_frame,dx,spy,do_flip)

 -- projectiles
 for p in all(projs) do
  local px,py=flr(p.x),flr(p.y)
  if p.exp then
   local r=p.et*2
   circ(px,py,r,7)
   if r>3 then circ(px,py,r-3,10) end
   if r>6 then circfill(px,py,r-6,9) end
  else
   -- small glowing energy ball
   circfill(px,py,2,10)
   circfill(px,py,1,7)
  end
 end

 local nf=acache[cur_anim].ai.nf
 print(anim_names[cur_anim].." "..cur_frame.."/"..nf,2,2,7)
 print("\x8d\x8e anim  z:flip",2,120,6)
end

