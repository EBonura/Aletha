pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- test: nibble extraction from packed bytes
-- packed byte = (hi_nibble << 4) | lo_nibble

function run_tests()
 local pass=0
 local fail=0

 function check(label,got,expected)
  if got==expected then
   printh("PASS "..label..": "..got)
   pass+=1
  else
   printh("FAIL "..label..": got "..got.." expected "..expected)
   print("FAIL "..label,0,fail*8,8)
   fail+=1
  end
 end

 -- test nibble extraction
 -- byte 0x6d = 109 = (6<<4)|13
 local b=0x6d  -- =109
 check("hi nibble raw b>>4",       b>>4,       6.8125)  -- shows the bug
 check("hi nibble fixed (b>>4)&0xf",(b>>4)&0xf, 6)      -- fix
 check("hi nibble fixed b\16",      b\16,       6)      -- alt fix
 check("lo nibble b&0xf",          b&0xf,      13)     -- lo nibble is fine

 -- byte 0xe7 = 231 = (14<<4)|7
 local b2=0xe7
 check("hi 0xe7 raw b>>4",          b2>>4,      14.4375) -- bug
 check("hi 0xe7 fixed (b>>4)&0xf", (b2>>4)&0xf, 14)    -- fix
 check("lo 0xe7 b&0xf",             b2&0xf,     7)

 -- byte 0x01 = (0<<4)|1
 local b3=0x01
 check("hi 0x01 raw b>>4",          b3>>4,      0.0625) -- bug
 check("hi 0x01 fixed (b>>4)&0xf", (b3>>4)&0xf, 0)    -- fix

 -- decode_rle round-trip test (bpp=2)
 -- encode manually: pixels=[0,0,0,1,2,3,0,0] -> 2bpp
 -- bpp=2: run_bits=6, color<<6|(run-1)
 -- [0]*3: (0<<6)|2=2, [1]*1: (1<<6)|0=64, [2]*1: (2<<6)|0=128, [3]*1: (3<<6)|0=192, [0]*2: (0<<6)|1=1
 local rle_data={2,64,128,192,1}
 local expected={0,0,0,1,2,3,0,0}
 local decoded=decode_rle_bpp(rle_data,8,2)
 local match=true
 for i=1,8 do
  if decoded[i]~=expected[i] then match=false end
 end
 check("decode_rle bpp=2 round trip", match and 1 or 0, 1)

 -- bpp=1 test: pixels=[0,0,0,1,0,1,1]
 -- run_bits=7, [0]*3: (0<<7)|2=2, [1]*1: (1<<7)|0=128, [0]*1: (0<<7)|0=0, [1]*2: (1<<7)|1=129
 local rle1={2,128,0,129}
 local exp1={0,0,0,1,0,1,1}
 local dec1=decode_rle_bpp(rle1,7,1)
 local m1=true
 for i=1,7 do if dec1[i]~=exp1[i] then m1=false end end
 check("decode_rle bpp=1 round trip", m1 and 1 or 0, 1)

 printh("done: "..pass.." pass, "..fail.." fail")
 print("done: "..pass.." pass "..fail.." fail",0,120,7)
end

function decode_rle_bpp(data,npix,bpp)
 local run_bits=8-bpp
 local run_mask=(1<<run_bits)-1
 local color_mask=(1<<bpp)-1
 local buf={}
 local idx=1
 local pos=1
 while idx<=npix do
  local b=data[pos]; pos+=1
  local color=(b>>run_bits)&color_mask
  local r=b&run_mask
  if r==run_mask then
   r=run_mask+1+data[pos]; pos+=1
  else
   r=r+1
  end
  for i=0,r-1 do buf[idx+i]=color end
  idx+=r
 end
 return buf
end

function _init()
 run_tests()
end

function _draw()
 -- results already printed in _init
end
