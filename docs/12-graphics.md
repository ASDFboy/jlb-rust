# 12. Graphics: windows, drawing and input

A program can open its own windows, draw shapes, text and images in them, and react to the keyboard and mouse.

```text
WINDOW(320, 180, "Bouncing ball", 3)
x = 20
speed = 2
WHILE WINDOW_OPEN() {
    x += speed
    IF x > 300 || x < 20 { speed = -speed }
    CLEAR("navy")
    CIRCLE(x, 90, 12, "orange")
    TEXT(10, 170, "Press Escape to stop", "white", [anchor: 1])
    IF KEY_PRESSED("escape") { WINDOW_CLOSE() }
    SHOW()
}
```

## The basics

- **Positions:** `(1, 1)` is the **bottom-left** pixel. `x` goes right and `y` goes up, so `(width, height)` is the top-right pixel. Fractions are fine, for smooth movement.
- **Angles** are in degrees, counterclockwise, with 0 pointing right (the same as `SIN`, `COS` and `ATAN`).
- **Frames:** drawing goes into the window's next picture. `SHOW()` puts it on screen, handles the keyboard and mouse, and waits so frames come `FPS()` times a second (60 unless changed). While it waits, tasks run, as with `WAIT`.
- **When the program's code ends,** its windows stay open (and their events keep working) until they're closed. `END` or an error closes them straight away.
- A loop that runs `SHOW()` ends the program when its window is closed, unless the loop asks `WINDOW_OPEN()` (then the loop decides, and the code after it runs).
- New types: `TYPE()` gives `"WINDOW"`, `"IMAGE"` and `"FONT"`. Windows and images have `.width` and `.height` (and windows `.title`).

## Colors

Anywhere a color goes, any of these works:

| Color | Example |
| --- | --- |
| a number `0xRRGGBB` | `0xFF8800` |
| `RGB(red, green, blue, [alpha])` | `RGB(255, 136, 0)`, `RGB(255, 136, 0, 128)` (half see-through) |
| a name | `"red"`, `"skyblue"`, `"transparent"`, `"#ff8800"`, `"#ff880080"` |
| a table | `[255, 136, 0]`, `[r: 255, g: 136, b: 0, alpha: 128]` |

Every amount goes from 0 to 255, and `alpha` 255 is solid. The names are `black`, `white`, `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `orange`, `purple`, `pink`, `brown`, `gray`, `lightgray`, `darkgray` (`grey` works too), `silver`, `gold`, `navy`, `teal`, `lime`, `maroon`, `olive`, `darkgreen`, `skyblue`, `lightblue`, `darkblue`, `darkred`, `violet`, `indigo`, `beige`, `tan`, `coral`, `salmon`, `crimson`, `turquoise`, `mint`, `lavender` and `transparent`.

`GET_PIXEL` gives colors back as numbers, so `GET_PIXEL(5, 5) == 0xFF0000` checks for red.

## Windows

| Command | What it does |
| --- | --- |
| `w = WINDOW(width, height, [title], [scale], [settings])` | opens a window `width` x `height` pixels big. `scale` 3 shows every pixel 3 times as big (retro pixel art). The title is the file's name unless given. |
| `DRAW_TO(w)` | with several windows: which one later commands draw into and read input from. With one window you never need it. |
| `SHOW()` | shows every window's new picture, handles events, and waits for the next frame |
| `FPS(n)`, `FPS()` | frames a second (0 = as fast as possible); `FPS()` gives it back |
| `WINDOW_OPEN([w])` | `TRUE` until the window is closed |
| `WINDOW_CLOSE([w])` | closes it |
| `WINDOW_TITLE(text)`, `WINDOW_TITLE()` | changes or gives back the title |
| `WINDOW_RESIZE(width, height)` | a new drawing size |
| `WINDOW_SET(settings)` | changes settings (below) |

Settings, for `WINDOW` or `WINDOW_SET`, like `WINDOW_SET([resizable: FALSE, cursor: "hand"])`:

| Setting | Meaning | Default |
| --- | --- | --- |
| `resizable` | the user can drag the window bigger or smaller (the picture is scaled to fit) | `TRUE` |
| `fullscreen_allowed` | F11 or Alt+Enter switches to full screen | `TRUE` |
| `fullscreen` | full screen now | `FALSE` |
| `title` | the title | the file's name |
| `cursor` | `"arrow"`, `"hand"`, `"text"`, `"crosshair"` or `"hidden"` | `"arrow"` |
| `icon` | an `IMAGE`, or an image file's name | |

## Options and styles

Every drawing command takes an optional **options table** as its last input, like `RECT(50, 50, 20, 10, "red", [angle: 45, anchor: 7])`. `STYLE([...])` sets options for every later command instead. An option in the call wins over the style, and the style wins over the default.

| Option | Meaning | Default |
| --- | --- | --- |
| `anchor` | which point of the shape sits at (x, y): 1 top-left, 2 top-middle, 3 top-right, 4 middle-left, 5 center, 6 middle-right, 7 bottom-left, 8 bottom-middle, 9 bottom-right | 5 |
| `angle` | turns it, in degrees counterclockwise | 0 |
| `pivot` | what it turns around: an anchor number 1 to 9, or a point `[x, y]` in the shape's own pixels (`[1, 1]` is its bottom-left) | the anchor |
| `outline` | draws just the edge, this thick (0 = filled) | 0 |
| `alpha` | how solid, 0 (invisible) to 255 | 255 |
| `scale`, `scale_x`, `scale_y` | images and text: bigger or smaller | 1 |
| `flip_x`, `flip_y` | images and text: mirrored | `FALSE` |
| `tint` | images and text: colored by this color | none |
| `smooth` | soft edges (anti-aliased); images are scaled smoothly too | `FALSE` |
| `radius` | `RECT`'s rounded corners | 0 |
| `font`, `size`, `align`, `wrap` | text (see below) | |

`STYLE()` gives the current style back as a table, so `old = STYLE()` ... `STYLE(old)` puts it back. `STYLE(table)` replaces the whole style, and `RESET_STYLE()` clears it.

## Shapes

The positions are where the shape's anchor goes (its center unless the `anchor` option says otherwise). Lines go between their points.

| Command | Draws |
| --- | --- |
| `CLEAR([color])` | the whole window (or image) in one color (black if none) |
| `PIXEL(x, y, color)` | one pixel |
| `GET_PIXEL(x, y)` | gives back the color there (`NULL` outside the picture) |
| `LINE(x1, y1, x2, y2, color, [width])` | a straight line |
| `ARROW(x1, y1, x2, y2, color, [width])` | a line with a point at (x2, y2) |
| `LINES(points, color, [width])` | lines joining the points, like `[[1, 1], [20, 30], [40, 1]]` |
| `CURVE(points, color, [width])` | a smooth curve through the points |
| `RECT(x, y, width, height, color, [outline])` | a rectangle (rounded with `radius`) |
| `CIRCLE(x, y, radius, color, [outline])` | a circle |
| `ELLIPSE(x, y, width, height, color, [outline])` | an oval |
| `TRIANGLE(x1, y1, x2, y2, x3, y3, color, [outline])` | a triangle |
| `POLYGON(points, color, [outline])` | a shape with corners at the points |
| `NGON(x, y, radius, sides, color, [outline])` | a regular shape: 6 sides is a hexagon |
| `STAR(x, y, outer, inner, points, color, [outline])` | a star with `points` points, `outer` and `inner` radius |
| `ARC(x, y, radius, start, end, color, [width])` | part of a circle's edge, from angle `start` to `end` |
| `PIE(x, y, radius, start, end, color, [outline])` | a pie slice |
| `FLOOD(x, y, color)` | paint bucket: fills the area of the same color around (x, y) |

Points can be `[x, y]` or `[x: 1, y: 2]`.

## Text

`TEXT(x, y, text, color, [size], [options])` writes text; `'\n'` starts a new line. The default font is a built-in smooth font, 16 pixels tall.

- `f = FONT("file.ttf")` loads a font file (any size). `FONT("pixel")` is a built-in crisp pixel font (sizes 8, 16, 24, ...).
- Options: `font`, `size`, `align` (`"left"`, `"center"` or `"right"`, for several lines) and `wrap` (a width to wrap the text at), plus the usual ones (`anchor`, `angle`, ...).
- `TEXT_SIZE(text, [size], [options])` gives back `[width: ..., height: ...]`.

## Images

| Command | What it does |
| --- | --- |
| `img = IMAGE("hero.png")` | loads a picture (PNG, JPG, BMP, GIF or TGA) |
| `IMAGE_NEW(width, height, [color])` | a new picture (see-through unless a color is given) |
| `PART(img, x, y, width, height)` | a piece of a picture, (x, y) its bottom-left pixel: for sprite sheets |
| `DRAW(img, x, y, [options])` | draws it, with any options: `scale`, `flip_x`, `angle` and `pivot`, `tint`, `alpha`, `anchor` ... |
| `DRAW_TO(img)` | later commands draw into the picture in `img` (`DRAW_TO()` goes back to the window) |
| `SAVE_IMAGE(img, "out.png")` | saves it as a PNG |
| `SCREENSHOT()` | a copy of the window's (or image's) picture so far |

Images copy like tables: after `b = a`, drawing into `b` doesn't change `a`. `DRAW_TO` needs a variable's name (like `DRAW_TO(background)`) so the drawing has somewhere to stay, and a `CONST` image can't be drawn into.

## Camera

`CAMERA(x, y, [zoom], [angle])` makes later drawing in the current window look at (x, y), in the middle of the window, zoomed and turned. `CAMERA()` goes back to plain window positions (for a score or menu drawn on top). `MOUSE()` gives the mouse's position both ways.

## Input, asked each frame

These describe the keyboard and mouse as of the last `SHOW()`, in the current window.

| Command | Gives back |
| --- | --- |
| `KEY_DOWN(key)` | `TRUE` while the key is held |
| `KEY_PRESSED(key)`, `KEY_RELEASED(key)` | `TRUE` in the frame it went down (or up) |
| `KEYS_DOWN()` | a table of the keys held |
| `TYPED()` | the text typed since the last frame (with Shift and the keyboard's layout) |
| `MOUSE()` | `[x:, y:, world_x:, world_y:, left:, right:, middle:, wheel:, inside:]`: the position in pixels, where that is for the camera, the buttons held, the wheel's turn this frame (up is positive), and whether it's over the window |
| `MOUSE_PRESSED([button])`, `MOUSE_RELEASED([button])` | `TRUE` in the frame the button (`"left"` unless given, `"right"` or `"middle"`) went down (or up) |

Keys are `"a"` to `"z"`, `"0"` to `"9"`, `"space"`, `"enter"`, `"escape"`, `"tab"`, `"backspace"`, `"delete"`, `"insert"`, `"left"`, `"right"`, `"up"`, `"down"`, `"home"`, `"end"`, `"pageup"`, `"pagedown"`, `"shift"`, `"ctrl"`, `"alt"`, `"f1"` to `"f12"`, punctuation as itself (`"-"`, `"["`, `","` ...), and `"any"` for any key.

## Input as events

A handler is an ordinary function, called when something happens. It's given as many of its inputs as it takes, and, like any function, uses `GLOBAL` to change outside names.

| Command | Calls the function |
| --- | --- |
| `ON_KEY(key, f)` | when the key goes down, with the key's name (holding it doesn't repeat) |
| `ON_KEY_UP(key, f)` | when it comes up |
| `ON_CLICK(f)` | with x, y and the button, when a mouse button goes down |
| `ON_MOUSE_MOVE(f)` | with x and y, when the mouse moves |
| `ON_TYPE(f)` | with each character typed |
| `ON_RESIZE(f)` | with the new width and height on screen |
| `ON_CLOSE(f)` | when the user closes the window: returning `FALSE` keeps it open |

`ON_KEY("any", f)` hears every key, and giving `NULL` instead of a function stops a handler. Handlers run during `SHOW()` (or while the windows stay open after the program's code has ended), in the order things happened, so they never run in the middle of other code.

```text
WINDOW(200, 200, "Click to draw", 2)
CLEAR("white")
DEF dot(x, y, button) {
    >> Draws where the mouse was clicked: red for left, blue for anything else.
    IF button == "left" { CIRCLE(x, y, 5, "red") } ELSE { CIRCLE(x, y, 5, "blue") }
}
ON_CLICK(dot)
ON_KEY("c", DEF() { CLEAR("white") })
```

## For tests

With the environment variable `JLB_HEADLESS=1`, windows draw into memory only: `SHOW()` closes them after `JLB_FRAMES` frames (1 unless set), and `JLB_SCREENSHOT=file.png` saves the first window's picture each time it's shown. Both interpreters draw exactly the same pixels, on every system.
