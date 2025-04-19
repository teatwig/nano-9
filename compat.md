# API Compatibility

- [x] _init()
- [x] _update()
- [x] _draw()
- [ ] flip()

## Graphics
- [x] camera([x,] [y])
- [x] circ(x, y, r, [col])
- [x] circfill(x, y, r, [col])
- [x] oval(x0, y0, x1, y1, [col])
- [x] ovalfill(x0, y0, x1, y1, [col])
- [ ] clip([x,] [y,] [w,] [h])
- [x] cls([col])
- [x] color(col)
- [x] cursor([x,] [y,] [col])
- [x] fget(n, [f])
- [ ] fillp([pat])
- [x] fset(n, [f,] [v])
- [x] line(x0, y0, x1, y1, [col])
- [x] pal([c0,] [c1,] [p])
- [x] palt([c,] [t])
- [ ] pget(x, y)
- [x] print(str, [x,] [y,] [col])
- [x] pset(x, y, [c])
- [x] rect(x0, y0, x1, y1, [col])
- [x] rectfill(x0, y0, x1, y1, [col])
- [x] sget(x, y)
- [x] spr(n, x, y, [w,] [h,] [flip_x,] [flip_y])
- [x] sset(x, y, [c])
- [x] sspr(sx, sy, sw, sh, dx, dy, [dw,] [dh,] [flip_x,] [flip_y])
- [ ] tline(x0, y0, x1, y1, mx, my, [mdx,] [mdy])

## Tables
- [x] add(t, v, [i])
- [x] all(t)
- [-] count(t, [v])
- [x] del(t, v)
- [x] deli(t, i)
- [x] foreach(t, f)
- [x] ipairs(t)
- [x] pack(...)
- [x] pairs(t)
- [ ] unpack(t, [i], [j])
- [x] next(t, [key])

## Input
- [x] btn([i,] [p])
- [x] btnp([i,] [p])

## Sound
- [ ] music([n,] [fade_len,] [channel_mask])
- [x] sfx(n, [channel,] [offset])

## Map
- [x] map(cel_x, cel_y, sx, sy, cel_w, cel_h, [layer])
- [ ] mget(x, y)
- [ ] mset(x, y, v)

## Memory
- [ ] cstore(destaddr, sourceaddr, len, [filename])
- [ ] memcpy(dest_addr, source_addr, len)
- [ ] memset(dest_addr, val, len)
- [ ] peek(addr, [n])
- [ ] peek2(addr, [n])
- [ ] peek4(addr, [n])
- [ ] poke(addr, [value,] [...])
- [ ] poke2(addr, [...])
- [ ] poke4(addr, [...])
- [ ] reload(destaddr, sourceaddr, len, [filename])
- [ ] serial(channel, sourceaddr, size)

## Math
- [x] abs(x)
- [x] atan2(dx, dy)
- [x] band(x, y)
- [x] bnot(x)
- [x] bor(x, y)
- [x] bxor(x, y)
- [x] ceil(x)
- [x] cos(x)
- [x] flr(x)
- [ ] lshr(num, bits)
- [x] max(x, y)
- [x] mid(x, y, z)
- [x] min(x, y)
- [x] rnd(x)
- [x] rotl(num, bits)
- [x] rotr(num, bits)
- [x] sgn(x)
- [x] shl(x, y)
- [x] shr(x, y)
- [x] sin(x)
- [x] sqrt(x)
- [x] srand(x)

## Cartridge data
- [ ] cartdata(id)
- [ ] dget(index)
- [ ] dset(index, value)
- [ ] cstore(dest_addr, source_addr, len, [filename])
- [ ] reload(dest_addr, source_addr, len, [filename])

## Coroutines
- [x] cocreate(func)
- [x] coresume(cor, [...])
- [x] costatus(cor)
- [x] yield([...])

## Strings
- [x] split(str, [separator, ] [convert_numbers])
- [x] sub(str, from, [to])
- [ ] chr(num)
- [x] ord(str, [index])
- [x] tonum(val, [format_flags])
- [-] tostr(val, [usehex])

## Values and objects
- [x] setmetatable(tbl, metatbl)
- [x] getmetatable(tbl)
- [x] rawequal(t1, t2)
- [x] rawget(t, n)
- [x] rawlen(t)
- [x] rawset(t, n, v)
- [x] select(i, ...)
- [x] type(v)

## Time
- [x] time() (alias: t())

## System
- [ ] menuitem(index, [label, callback])
- [ ] extcmd(cmd)
- [ ] run([breadcrumb])

## Debugging
- [ ] assert(cond, [message])
- [-] printh(str, [filename], [overwrite])
- [ ] stat(n)
- [ ] stop() (undocumented)
- [ ] trace() (undocumented)

