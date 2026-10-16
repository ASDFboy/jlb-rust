# 10. Built-in commands

Built-ins are single ALL-CAPS words. An input shown in `[brackets]` is optional; passing `NULL` for it also means "use the default".

## General rules

- **All time values are in seconds.**
- **Commands that are actions change what you give them, and also give back the result:** `INSERT`, `DROP`, `UNDEFINE`.
- **Commands that compute something give back a new copy** and leave the original alone: `SORT`, `ALPHA_SORT`, `KEYS`, `UPPER`, `LOWER`, and the rest.

## Output, input and files

| Command | What it does |
| --- | --- |
| `PRINT(a, b, ...)` | prints its inputs separated by spaces, then a new line. Text prints without quotes. |
| `INPUT([prompt])` | shows the prompt, then gives back the typed line as text |
| `READ(path)` | the whole file as text |
| `READ_LINES(path)` | a table with one item per line |
| `WRITE(path, value)` | replaces the file's contents (creating it if needed) |
| `EXISTS(path)` | TRUE or FALSE |

## Conversion and inspection

| Command | What it does |
| --- | --- |
| `NUM(x)` | text to a number (any [number format](03-values-and-types.md#which-text-counts-as-a-number)); `TRUE` gives 1, `FALSE` and `NULL` give 0 |
| `STR(x)` | the value as text, written the way `PRINT` shows it |
| `ENG(n)` | engineering notation: `ENG(1500)` is `"1.5e3"`, `ENG(0.002)` is `"2e-3"` |
| `HEX(n)` | hex text for a whole number: `HEX(255)` is `"0xFF"` |
| `TYPE(x)` | `"NUMBER"`, `"TEXT"`, `"BOOL"`, `"NULL"`, `"TABLE"`, `"FUNCTION"` or `"TASK"` |
| `LEN(x)` | the numbered items in a table, the characters in text, or the characters in a number written as text (`LEN(-2.5)` is 4) |

## Tables and text

| Command | What it does |
| --- | --- |
| `INSERT(t, value, pos)` | inserts so the value ends up at `pos` (negative counts from the end); works on text too |
| `DROP(t, pos)` | removes the item or character at `pos` |
| `UNDEFINE(name)` | removes a variable, function, table or table entry |
| `KEYS(t)` | positions, then names: `KEYS(["a", name: 1])` is `[1, "name"]` |
| `SORT(t)` | sorted copy, smallest first, using the `<` rules (text sorts by length). For biggest first: `SORT(t) * -1`. |
| `ALPHA_SORT(t)` | sorted copy in alphabetical order (capitals only break ties) |
| `FIND(text, part)` | position of the first match, or NULL: `FIND("hello", "l")` is 3 |
| `FIND(t, value, [depth])` | position or name of the first matching item, or NULL. `depth` is how many levels of tables-inside-tables to search (default 0). A match inside a sub-table gives a path: `["inv", 2]`. Matches use `===`, except tables, which match by contents. |
| `SPLIT(text, sep)` | splits text into a table; `""` as `sep` splits into characters |
| `JOIN(t, [sep])` | joins the numbered items into text |
| `UPPER(x)`, `LOWER(x)` | changes the case |

## Math

| Command | What it does |
| --- | --- |
| `ABS(x)` | removes the minus sign |
| `MIN(a, b, ...)`, `MAX(a, b, ...)` | smallest or largest of any number of values |
| `TABLE_MIN(t, [depth])`, `TABLE_MAX(t, [depth])` | smallest or largest value in a table, searching `depth` levels of sub-tables |
| `ROUND(x, [decimals])`, `FLOOR(x, [decimals])`, `CEIL(x, [decimals])` | round to nearest, down or up; whole numbers unless `decimals` (0 to 1000) is given. The result is always exact, and halves round away from zero: `ROUND(2.5)` is 3, `ROUND(-2.5)` is -3 |
| `PI([decimals])` | `PI()` is 3.141592653589793 (approximate); `PI(2)` is 3.14, and `PI(decimals)` gives up to 100 exact decimal places |
| `RANDOM(min, max, [decimals], [seed])` | random number from min to max, including both (see below) |

### Trigonometry

| Command | What it does |
| --- | --- |
| `SIN(x)`, `COS(x)`, `TAN(x)` | sine, cosine, tangent |
| `CSC(x)`, `SEC(x)`, `COT(x)` | cosecant, secant, cotangent |
| `ASIN(x)`, `ACOS(x)` | the angle whose sine or cosine is x |
| `ATAN(y, [x])` | the angle from one point to another (with one input, the plain arctangent) |

- **Trig uses degrees:** `SIN(90)` is 1, and `ATAN(1, 1)` is 45.
- **Every trig command has a radians version** with `RAD_` in front: `RAD_SIN`, `RAD_COS`, `RAD_TAN`, `RAD_CSC`, `RAD_SEC`, `RAD_COT`, `RAD_ASIN`, `RAD_ACOS` and `RAD_ATAN`. For example, `RAD_SIN(PI() / 2)` is 1.
- **Degree results are rounded to 12 decimal places,** so `COS(90)` is exactly 0 and `SIN(30)` is exactly 0.5, not tiny leftovers. Radian results are approximate with full precision (about 16 digits), so `RAD_ATAN(1, 1) == PI() / 4` is TRUE.
- **Undefined points are errors:** `TAN(90)` is an error.

### RANDOM

- **With just min and max,** it gives a whole number: `RANDOM(1, 10)` gives 1 to 10.
- **If min or max has decimals,** the result has that many decimal places: `RANDOM(1, 2.5)` gives a number like `1.7`.
- **The third input sets the decimal places directly:** `RANDOM(1, 10, 2)` gives a number like `4.37`.
- **The fourth input is a seed.** Without one, results differ every run. With one, the same inputs always give the same result: `RANDOM(1, 10, NULL, 42)`.

## Time

| Command | What it does |
| --- | --- |
| `TIME([decimals])` | seconds since the program started |
| `DELTA_TIME([decimals])` | seconds since the last `DELTA_TIME()` call (the first call counts from the start) |
| `WAIT(seconds)` | pauses the program (while [tasks](11-tasks.md) are running, the others carry on) |

## Tasks

| Command | What it does |
| --- | --- |
| `START f(x)` | runs a call as a task and gives back the task (see [Tasks](11-tasks.md)) |
| `AWAIT(task)` | waits for a task (or a table of them) and gives back what it returned |
| `DONE(task)` | `TRUE` once the task has finished |
| `CANCEL(task)` | stops a task |

## Graphics

Windows, drawing, text, images and input: `WINDOW`, `SHOW`, `CLEAR`, `RECT`, `CIRCLE`, `TEXT`, `IMAGE`, `DRAW`, `KEY_DOWN`, `MOUSE`, `ON_CLICK` and the rest are all in [Graphics](12-graphics.md).

## Errors and stopping

| Command | What it does |
| --- | --- |
| `ERROR(message)` | raises an error (see [Errors](08-errors.md)) |
| `END` | stops the program immediately, without a report |
