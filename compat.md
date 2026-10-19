# API Compatibility

Parameters shown in italics are extensions to the Pico-8 API. 

## Hooks

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
- [x] fillp([pat])
- [x] fset(n, [f,] [v])
- [x] line(x0, y0, x1, y1, [col])
- [/] pal([c0,] [c1,] [p])

`pal()` resets the draw map and the display map. `pal(0)` resets only the draw
map used by later indexed sprites. `pal(1)` resets the display map.
`pal(c0, c1)` and `pal(c0, c1, 0)` remap later `spr` and `sspr` draws.
`pal(c0, c1, 1)` recomposes each live indexed-sprite LUT as `display[draw[i]]`,
leaving draw transparency on that LUT, so earlier sprites keep their own draw
map. That is sprite lookup only: `cls`, primitives, text, `map`, baked `tline`,
and canvas pixels are left alone. `pal(2)` and `pal(..., 2)` are not supported.
`palm` selects an asset palette.

- [x] palt([c,] [t])
- [/] pget(x, y)

Reads visible pixel canvases from the top down and returns the first pixel
`pset` wrote. A hole in a newer canvas shows an older one. A coordinate no
canvas has written, or one outside the canvas, returns nil. Color 0 from `pset`
is a pixel, not a hole. `rect`, `spr`, `map`, and `print` are not in those
buffers.

- [x] print(str, [x,] [y,] [col,] *[font_size]*)
- [x] pset(x, y, [c])
- [x] rect(x0, y0, x1, y1, [col])
- [x] rectfill(x0, y0, x1, y1, [col])
- [x] rrect(x, y, w, h, [r,] [col])
- [x] rrectfill(x, y, w, h, [r,] [col])
- [x] sget(x, y)
- [x] spr(n, x, y, [w,] [h,] [flip_x,] [flip_y,] *[turns]*)
- [x] sset(x, y, [c])
- [x] sspr(sx, sy, sw, sh, dx, dy, [dw,] [dh,] [flip_x,] [flip_y,] *[image_index]*)
- [/] tline(x0, y0, x1, y1, mx, my, [mdx,] [mdy], [layers,]*[image_index]*)

This is only partially implemented. It works on the image currently rather than
the map. But it will be made to work with maps soon.

## Tables
- [x] add(t, v, [i])
- [x] all(t)
- [x] count(t, [v])
- [x] del(t, v)
- [x] deli(t, i)
- [x] foreach(t, f)
- [x] ipairs(t)
- [x] pack(...)
- [x] pairs(t)
- [x] unpack(t, [i], [j])
- [x] next(t, [key])

## Input
- [x] btn([i,] [p])
- [x] btnp([i,] [p])

## Sound
- [/] music([n,] [fade_len,] [channel_mask,] *[bank_index]*)

Plays cart patterns, including loop and stop. `fade_len` is ignored. A set bit in `channel_mask` is stored and does not change which pattern channels play, and `sfx()` does not consult it.

- [/] sfx(n, [channel,] [offset,] [length,] *[bank_index]*)

`length` is accepted and then rejected.

## Map
- [x] map(cel_x, cel_y, sx, sy, cel_w, cel_h, [layers], *[map_index]*)
- [x] mget(x, y)
- [x] mset(x, y, v)

## Memory

There is no 64 KiB address space. Sprite, map, flag, music, and sfx bytes live as assets. `peek` and `poke` do not reach those regions, the screen, general-use RAM, cart data, GPIO, or the hardware block. `peek(addr, n)` has no length argument. `poke` takes one byte.

- [/] peek(addr, [n])

| address | Pico-8 | Nano-9 |
|---------|--------|--------|
| 0x5f27  | print cursor y | returns `draw_state.print_cursor.y` |

Any other address is an error. The old note listed `0xf527`; the implemented address is `0x5f27`.

- [/] poke(addr, [value,] [...])

| address | Pico-8 | Nano-9 |
|---------|--------|--------|
| 0x5f2d  | devkit flags: bit 0 keyboard, bit 1 mouse buttons as `btn(4)`..`btn(6)`, bit 2 pointer lock | any non-zero value enables keyboard input. The other bits are ignored. |

- [ ] peek2(addr, [n])
- [ ] peek4(addr, [n])
- [ ] poke2(addr, [...])
- [ ] poke4(addr, [...])
- [ ] memcpy(dest_addr, source_addr, len)
- [ ] memset(dest_addr, val, len)
- [ ] cstore(destaddr, sourceaddr, len, [filename])
- [ ] reload(destaddr, sourceaddr, len, [filename])
- [ ] serial(channel, sourceaddr, size)

Pico-8 regions with no `peek` or `poke` path:

| range | purpose |
|-------|---------|
| 0x0000–0x0fff | sprite sheet, upper half |
| 0x1000–0x1fff | sprite sheet lower half, or map rows 32–63 |
| 0x2000–0x2fff | map rows 0–31 |
| 0x3000–0x30ff | sprite flags |
| 0x3100–0x31ff | music patterns |
| 0x3200–0x42ff | sfx |
| 0x4300–0x55ff | general use |
| 0x5600–0x5dff | general use / custom font |
| 0x5e00–0x5eff | persistent cart data |
| 0x5f00–0x5f1f | draw palette and screen palette |
| 0x5f20–0x5f3f | draw state, aside from the one peek above |
| 0x5f40–0x5f7f | hardware state (audio effects, RNG, button bits, map and screen mapping) |
| 0x5f80–0x5fff | GPIO |
| 0x6000–0x7fff | screen |
| 0x8000–0xffff | general use / extended map |

These draw-state fields exist on `DrawState` and still have no `peek` or `poke`:

| address | field |
|---------|--------|
| 0x5f25 | pen color |
| 0x5f26 | print cursor x |
| 0x5f28–0x5f2b | camera x and y, 16-bit little-endian |
| 0x5f31–0x5f32 | fill pattern |

Clip rect (`0x5f20`–`0x5f23`), screen transform (`0x5f2c`), fill-pattern flags (`0x5f33`), and the previous line endpoint (`0x5f3c`–`0x5f3f`) have no field.

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
- [x] lshr(num, bits)
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
- [/] extcmd(cmd)
- [ ] run([breadcrumb], [param])
- [/] load(filename, [breadcrumb], [param])

`load()` stores the breadcrumb (`stat(100)`) and parameter string (`stat(6)`)
and queues a cart swap. The CLI `-p` / `--param` flag provides `stat(6)` the
same way Pico-8's `-p` does. The load dialog (no filename) is not implemented.

## Debugging
- [ ] assert(cond, [message])
- [/] printh(str, [filename], [overwrite])
- [/] stat(n)
Supports keyboard and some mouse functions.

| value | description                                      |
|-------|--------------------------------------------------|
| 6     | parameter string from `load()` or CLI `-p`       |
| 8     | target frame rate                                |
| 9     | actual frame rate                                |
| 30    | has keyboard input                               |
| 31    | keyboard input if any                            |
| 32    | mouse position x                                 |
| 33    | mouse position y                                 |
| 34    | mouse buttons                                    |
| 100   | breadcrumb string from `load()`                  |
- [ ] stop() (undocumented)

The Lua function prints `STOP NOT IMPLEMENTED` and returns.
- [ ] trace() (undocumented)

# Performance Compatability

Let me provide an example where Nano-9 and Pico-8 performance differs. In
Pico-8 if one doesn't clear the screen `cls()` and continues to draw sprites
`spr()` each `_draw()` call, the performance curve will be flat. However, in
Nano-9 a `spr()` creates a Bevy `Sprite` and if one doesn't clear them
frequently, they will accumulate and degrade performance.

Why not reify the last render to an image to preserve Pico-8's performance?
One could do this certainly but the aim is to support Bevy's native elements
as much as possible. I'd prefer for `spr()` to be a thin layer to Bevy's
[`Sprite`](https://docs.rs/bevy/latest/bevy/sprite/struct.Sprite.html),
`print()` to create a
[`Text`](https://docs.rs/bevy/latest/bevy/prelude/struct.Text.html) component,
`map()` to create a
[`bevy_ecs_tilemap::TilemapBundle`](https://docs.rs/bevy_ecs_tilemap/latest/bevy_ecs_tilemap/type.TilemapBundle.html).

In this way the comfortable world of Pico-8 can help introduce one to the
world of Bevy, and it can also provide affordances not possible in Pico-8. For
instance one could query on-screen entities for collision information with a
suitable extension.
