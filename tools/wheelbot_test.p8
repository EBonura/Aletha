pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
--##generated##
trans=14
a_wbi,a_wbm,a_wbc,a_wbs,a_wbfd,a_wbwk,a_wbd,a_wbdt=1,2,3,4,5,6,7,8
wheelbot_base=17152 wheelbot_cw=112 wheelbot_ch=26
wheelbot_data="\008p\026\000\0008\000\206\001r\002\180\003\217\006\236\007~\008\001\001\002\224`\000\000\010\008\013\017\004D\006F\005F\003\129F\002\128\001F\001\128\001H\000\128\000J\128\000J\128\000J\000\128J\001J\001J\001J\001J\001J\001J\002H\000\008\000\002\224g\003\004\002\027\023\000\002\003P\000I\000K\000\000\002\001\002\000\002\001\002\228\000\229\000\249\000\250\000\251\000\020\001;\001T\001\011D\020E\020F\017\129B\129A\016\128\001B\129@\128\015\128\002E\128\015\128\002F\014\128\001H\014\128\000I\014\128\000J\013\128\000E\139\007\128D\131A\134\008E\129C\131\010K\014J\015J\015J\015J\016I\018F\012\192\005D\013\194\005B\013\198\017?\030B\021E\019F\019B\129@\128\018\128B\129@\128\017\128\000F\016\128\001F\015\128\002G\014\128\001J\012\128\001E\139\006\128\000D\131A\134\006\128\000E\129C\131\009\128K\014J\015J\015J\015J\015J\015I\007\192\008E\010?\002D\020F\019F\017\129B\129@\128\016\128\001B\129@\128\015\128\002F\015\128\002F\015\128\001H\014\128\000J\013\128\000J\013\128\000D\139\008\128C\131A\134\009D\129C\131\011J\015J\015J\015J\015J\016H\018D\017\193\002B\011\000\011u\000\002\242\010\000\242\009\000\243\255\142\001\243\010\003\0030\000\000\000\021\026\001\002\002\160\000\194\002!\001\002\002P\000\210\002\001\001\"\002 \000\240\007\031u\000\002\242\010\000\242\009\000\242\150\002\160\000\194\002!\001\002\002P\000\210\002\001\001\"\002 \000\243\152\243\010\003\0030\000\021Q\001\002\002\160\000\194\002!\001\002\002P\000\210\002\001\001\"\002 \000\240\007\020P\001\002\002\160\000\194\002!\001\002\002P\000\210\002\001\001\"\002 \000\004\002\003\224g\208\000\010\003\023\022\000\000R\000k\000\128\000\004$\003`\011&\005`\008&\013A%@\012@\001\"A\128j\001@\002#\128K`\000@\002\"\128C!Fa@\001%A#Cc@\000'@#d\002@\000+\008@\000*\010@*\011*\011*\011*\011*\011*\011*\012(\015$\017$\018\"\013\013`\024`\017a\005`\012c\003a\021a\021`\021`\020a\018`\031\255\014\031\005`\005`\017`\001`\013f\000b\020a\021`\031\255\0311\031\008`\017a\002`\000a\016b\002`\031\255\031_\004\000\002\224g\002\009\000$\025\000\003b\000Q\000\000\001\001\001\179\000\180\000\014\001%\001?2D\014\195\010F\011\199\008F\010\201\005\129E\128\009\203\003\128\001B\129@\128\008\204\002\128\002F\006\207\001\128\002\138\192\000\208\001\128\001\131A\134\000\208\001\128\000A\129C\131\003\207\001\128K\006\204\002\128K\007\203\003\128J\008\201\005J\009\199\006J\011\195\008J\024J\024J\024J\025H\028D\030D\031B\025\025\192?\023D\021\192\006F\028F\007\192\017\129E\128\025\128\001B\129@\128\024\128\002F\024\128\002A\139\017\128\001A\131A\134\017\128\000C\129C\131\019\128\000J\022\128\000J\020\192\001\128J\024J\024J\024J\024J\024J\024J\025H\028D\030D\031B\025\000J\026\000\000\2438\225@\243\002\0030\208\195S\019q`\128#\147P\001R\000\147\003\242\001\001\"\017\002\000\131\019\163a`\243\007\018\000!\018\147\016\243\000\018\00021\018`\243\000@\"\01720C\195\243\019\243\020\016\018\000\243\004\003\131R\243\006\003C\003\243\015\019\019\241\137\241\021\001\011\026\000\000\243\020\240?\243\020\240\007\243\020\243X\243\146\000\243+\243@\000\007\000\002\224g\004\017\003_\023\000\001\002\0066\000\158\000\204\000c\000\000\001\002\003\003\003\003\003\002\004\002\005\002\006\002\181\002\219\002\249\002?\255?\175\192?\028\194?\027\195?\025\197\000\192?\019\203?\017\205?\013\209?\011\211?\007\215?\001\2209\2281\236*\243\029\255\001\021\255\008\011\255\019\010\255\019?+?\014D?\025F?\024F?\024F\129?\022\128@\129B\001\128?\021F\002\128?\020F\002\128?\008\139G\001\128?\008\134A\131G\000\128?\010\131C\129H\000\128?\014N\000\128?\003\192\010A\192J\128\192*\192\001\192\016\196\012J\194\030\193\007\198\013\197\012J\194\029\195\005\200\004\192\005\197\013I\195\028\196\005\199\004\195\004\195\014I\196\026\196\007\197\004\197\004\192\016H\197\026\197\007\195\005\198\021F\199\025\198\016\200\013\193\005D\200\024\201\013\203\011\195\005B\201\020\206\009\211\004\197\004\205\005\255\025?\"?\015D?\025F?\024F?\024F\129?\022\128@\129B\001\128?\021F\002\128?\020F\002\128?\008\139G\001\128?\008\134A\131G\000\128?\010\131C\129H\000\128?\014N\000\128 \193\005\193\004\194\013\194\010A\000J\128\000\192\030\195\003\195\002\194\004\192\007\196\012J\000\192\000\192\028\196\003\194\002\195\002\195\005\192\002\192\013J\000\195\026\192\001\193\005\192\004\193\002\196\024G\192A\001\194\024\192\016\193\003\195\025E\195@\001\195\022\193\016\192\004\196\024E\194A\001\195\022\192\022\198\015\192\006E\193B\002\195\020\192\008\192\012\195\001\192\016\193\006H\003\195\029\194\010\192\022\192\001\192\007D\005\194 \193%\193\005D\006\192\012\195\022\196\013\192\004\192\016A\194?)?\017D?\025F?\024F?\024\128@\129B\129?\022\128@\129B\001\128?\021F\002\128?\020F\002\128?\008\139G\001\128?\008\134A\131G\000\128?\010\131C\129H\000\128?\014N\000\128?\015A\000J\128?\019J?\020J?\020J?\020J?\020J?\020J?\021H?\024D?\026D?\027B?)\000\000\000\138P\000\001@\241I`\241H`\242H\001\002\017\"\016\242F\001\002\017 \018\000\241E`\"\000\241D`\"\000\2428\177p\018\000\2428a\0181p\002\000\243\019\211\243\000\13012\017\128\002\000\243\018\003Sc\003S\131\003\003\177\224\002\003\243\016\003\003S\019C\003S\003\003c#\193\016\001\162\003\003\003\243\016\003\003\243\002\019\003\241\010s#\003\003\243\018\243\005\241\011c\003 \243\013\241'S\0030C\243\007\243\031\129S@C\2436\003q\1603\003\003\2435\003q\1603\003\243\017\243\020\161\128\243\024\241 @\241J@\243\006\243\011\243\004\241\006 \243\007\022\022\003\003\211\243\000\2430\019Ccc\163\243.\131\211\243\024\242I\243\205\243\031\243P\243\009\243\207\243\154\243\011\243\005\014\183\002\003\243P\179\243\002\2437\211\243\024\243\2555\001\243\127\243\009\243\207\243\154\243\011\243\005\000\005\000\002\224`\003\010\002\021\023\000\001\0035\000U\000J\000\000\001\002\002\002\212\000\213\000\214\000\230\000\231\000?.D\014F\013F\011\129F\010\128\001B\129@\128\009\128\001H\008\128\000J\007\128\000J\007\128\000J\008\128J\009J\009J\009J\009J\008L\007L\009H\014B\011\004D\014F\013F\012\128B\129@\128\011\128\000B\129@\128\010\128\001F\009\128\002F\009\128\001H\008\128\000J\007\128\000J\007\128\000J\008\128G\130\009H\130\008G\130@\129\006L\129\004M\131\002M\132\002J\000\134\002H\001\134\004D\002\133\006D\002\132\008B\003\128\027\025D\014F\013F\011\129B\129@\128\010\128\001B\129@\128\009\128\002F\009\128\002F\009\128\001H\008\128\000J\007\128\000J\007\128\000J\008\128E\139\002D\131A\134\002E\129C\131\004K\008J\009J\009J\010H\013D\015D\016B\011\000\000\009A\000\002\242\004\000\242\003\000\242\190\002\242\003\002\000\005V\000\000\240\004\002\240\003\002\002\001\002\224`\000\000N\000\010\003\020\022\004D\013F\012F\011\128B\129@\128\010\128\000B\129@\128\009\128\001F\008\128\002F\008\128\001H\007\128\000J\006\128\000J\006\128\000J\007\128E\139\001D\131A\134\001E\129C\131\003K\007J\008J\008J\009H\012D\014D\015B\010\010\005\021\020\006\130\015\133\013\134\013\134\012\135\011\128\000\134\010\128\001\134\009\128\002\135\008\128\001\138\006\128\001\145\000\128\000\145\000\128\000\143\003\140\008\138\009\138\009\138\009\138\009\138\009\137\011\133\010\006\000\002\224`\004\000\005\029\020\000\001\002\003;\000;\0000\000%\000\000\001\002\003\003\003\203\000\204\000\205\000\206\000\207\000\227\000?:B\129\022C\129\022A\129B\000A\018A\129F\017J\139\005I\131A\134\005\128I\129C\131\007\128\000N\011\128\001L\012\128\001K\013\128\002J\014\132G\020G\021E\022D\024B\012\021\130\024\132\023\133\022\134\020\131\000\131\019\130\002\129\020\130\022\128\000\131\021\129\000\130\016E\133\016E\132\017F\131\016H\130\011Q\009R\009R\008B\128O\008B\128O\008S\009A\128N\007?*\128\024\132\022\131\023\131\025\131\025\131\017G\130\017H\130\016H\131\014J\131\008L\134\007N\133\007R\008B\128O\008B\128O\008S\009A\128N\008?\205I\018J\017J\016K\011Q\009R\009R\008B\128O\008B\128G\139\004J\131A\134\005A\128G\129C\131\006\000\000\000\000\014\225\001\002\002\002\002\002\002\242\002\241\019@\000\241\008\000\000\000\000"
_wa=split("16|17,17,17,20,16,16,16,19|21,21,21,20|26,26,26,27|59,64,68,70,70,70,96|16,20,19,19,19|19,20|16,15,11,11,11,11","|",false)
wb_anc={} for i=1,#_wa do wb_anc[i]=split(_wa[i]) end
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

anim_names={"idle","move","charge","shoot","firedash","wake","damaged","death"}
cur_anim=1
cur_frame=1
frame_timer=0
frame_spd=6
do_flip=false

-- wheel bot screen position (centered)
wbx=8
wby=90
floor_y=116

function _init()
 palt(0,false)
 palt(trans,true)
 local p=wheelbot_base
 for i=1,#wheelbot_data do poke(p,ord(wheelbot_data,i)) p+=1 end
 local wna=peek(wheelbot_base)
 for a=1,wna do
  local ai=read_anim(a,wheelbot_base)
  acache[a]={ai=ai,frames=decode_anim(ai),cw=wheelbot_cw,ch=wheelbot_ch}
 end
end

function _update()
 if btnp(0) then
  cur_anim-=1
  if cur_anim<1 then cur_anim=#anim_names end
  cur_frame=1
  frame_timer=0
 end
 if btnp(1) then
  cur_anim+=1
  if cur_anim>#anim_names then cur_anim=1 end
  cur_frame=1
  frame_timer=0
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
end

function _draw()
 cls(5)
 -- floor
 rectfill(0,floor_y,127,127,1)
 line(0,floor_y-1,127,floor_y-1,13)

 -- wheel bot (anchor-based positioning)
 local ax=(wb_anc[cur_anim] and wb_anc[cur_anim][cur_frame]) or wheelbot_cw\2
 local dx
 if do_flip then
  dx=wbx-(wheelbot_cw-1-ax)
 else
  dx=wbx-ax
 end
 draw_char(cur_anim,cur_frame,dx,wby,do_flip)

 -- center line (where wbx is — should stay consistent when flipping)
 line(wbx,wby-2,wbx,wby+wheelbot_ch+2,11)

 -- bbox outline
 local buf,bx,by,bw,bh=get_frame(cur_anim,cur_frame)
 if bw>0 then
  local bxd=bx
  if do_flip then bxd=wheelbot_cw-bx-bw end
  rect(dx+bxd-1,wby+by-1,dx+bxd+bw,wby+by+bh,8)
 end

 local nf=acache[cur_anim].ai.nf
 print(anim_names[cur_anim].." "..cur_frame.."/"..nf,2,2,7)
 print("cell:"..wheelbot_cw.."x"..wheelbot_ch,2,10,6)
 if bw>0 then
  print("bbox:"..bx..","..by.." "..bw.."x"..bh,2,18,6)
 end
 print("anc:"..ax.." flip:"..(do_flip and "y" or "n"),2,26,6)
 print("\x8d\x8e anim  z:flip",2,120,6)
end

