# 3. Values and types

Variables need no declaration. A variable is created when it's first assigned, and it can hold any type at any time:

```text
value = 10
value = "hello"      >> fine: the type can change
```

### Constants

`CONST` gives a name a value that can never change:

```text
CONST max_hp = 100
CONST start = [x: 0, y: 0]
max_hp = 50          >> error: max_hp is a constant (CONST), so it can't be changed
start.x = 3          >> error: a constant's tables can't be changed either
pos = start          >> a copy, which can be changed like any other table
pos.x = 3
```

- **Nothing about a constant changes:** not with `=`, `+=` and the others, `@=`, `UNDEFINE`, `INSERT` or `DROP`, and not through a name that shares it with `@`.
- **CONST makes a new name.** It's an error if the name already has a value in the same place (the file's outside names, or a function's own locals).
- **Inside a function, CONST makes a local constant,** like `LOCAL`: it's gone when the function finishes.
- **A copy of a constant is an ordinary value.** `LOAD` gives copies too, so `tools.max_hp` can be changed in your copy without touching the file's constant.

`TYPE(x)` tells you what something is:

| Type | Examples | `TYPE` gives |
| --- | --- | --- |
| number | `10`, `2.5`, `-3`, `1e3`, `0xFF` | `"NUMBER"` |
| text | `"hello"`, `'it\'s'` | `"TEXT"` |
| true/false | `TRUE`, `FALSE` | `"BOOL"` |
| nothing | `NULL` | `"NULL"` |
| table | `[1, 2]`, `[name: "Alex"]` | `"TABLE"` |
| function | `DEF(x) { RETURN x }`, `PRINT` | `"FUNCTION"` |
| task | `START f()` (see [Tasks](11-tasks.md)) | `"TASK"` |

## Numbers

There's one number type. Numbers show without decimals they don't need: `2.0` displays as `2`, and `5 / 2.5` gives `2`. Results that need decimals keep them: `5 / 2` gives `2.5`.

You can write numbers in these forms:

| Form | Examples |
| --- | --- |
| whole | `25`, `0` |
| decimal | `2.5`, `0.5` |
| engineering / scientific | `1e3`, `1.5e-3` |
| hex | `0xFF`, `0x1a` |

`025` (a leading zero) is an error, and so is `.5` (write `0.5`). A negative number is just a minus sign in front: `-5`.

### Numbers are exact

Numbers don't pick up rounding errors, and they grow to as many digits as they need:

```text
0.1 + 0.2             >> 0.3 (and 0.1 + 0.2 == 0.3 is TRUE)
19.99 * 3             >> 59.97
1 / 3 * 3             >> 1
2 ^ 100               >> 1267650600228229401496703205376
1 / 2 ^ 10            >> 0.0009765625
```

How it works:

- **Whole numbers can be as big as you need** (up to a million digits). Small ones use a fast 64-bit form, and bigger ones switch to as many digits as they need, automatically.
- **Decimals are stored as exact fractions.** `0.1` is exactly one tenth, so adding, subtracting, multiplying and dividing never drifts.
- **Decimals that go on forever,** like `1 / 3`, are kept as exact fractions too. `PRINT` shows them with 16 significant decimals (`0.3333333333333333`), but the real value is still exactly one third.
- **Long calculations stay fast.** If a forever-decimal's fraction gets very long (its bottom number passes 50 digits), it's rounded to 50 significant decimals. That only happens in long chains of division, like repeating `x = (x + 2 / x) / 2` to find a square root, and the error is far too small to show.
- **Some answers can't be written exactly,** because their digits never repeat: most roots (`2 !^ 2` is `1.4142135623730951`), powers like `2 ^ 0.5`, trig in radians, and `PI()`. Those are approximate, with about 16 significant digits, so `2 !^ 2 * 2 !^ 2` shows `2.0000000000000004`. Roots that come out even are exact (`16 !^ 2` is `4`, `0.25 !^ 2` is `0.5`), and `ROUND` turns an approximate number back into an exact one.
- **The size limit is a million digits.** Going past it is an error: `that number needs too many digits to store`.

## Text

Text goes in quotes, and the two kinds of quotes behave differently:

| Quotes | Behaviour |
| --- | --- |
| `"double"` | exactly as written. A backslash is just a backslash, and a `"` can't appear inside. |
| `'single'` | backslash codes become special characters |

Backslash codes in single quotes:

| Write | Means |
| --- | --- |
| `\'` | a single quote |
| `\"` | a double quote |
| `\n` | a new line |
| `\t` | a tab |
| `\\` | a backslash |

Any other backslash code is an error. Both kinds of quotes can span several lines, and the line breaks become part of the text.

```text
PRINT("C:\games\new")          >> C:\games\new
PRINT('Line one\nLine two')    >> two lines
PRINT('She said "hi"')         >> She said "hi"
PRINT('don\'t')                >> don't
PRINT("don't")                 >> don't
```

**Text that looks like a number is still text.** Math operators never convert it automatically:

```text
"25" + 2       >> "252"   (joins)
"25" * 2       >> "2525"  (repeats)
NUM("25") * 2  >> 50      (convert first to do math)
```

## TRUE and FALSE

`TRUE` and `FALSE` are their own values, not just `1` and `0`. Comparisons produce them, and `PRINT(5 > 3)` shows `TRUE`. They're loosely equal to 1 and 0 (`TRUE == 1` is TRUE), but not strictly (`TRUE === 1` is FALSE).

## NULL

`NULL` means "no value at all". You get it from:

- a function that ends without `RETURN`;
- reading a table position or name that doesn't exist;
- a `LOCAL` variable that hasn't been given a value yet.

NULL is forgiving in most places:

| Where | NULL acts as |
| --- | --- |
| math | `0`: `NULL + 1` is `1` |
| joining text | nothing: `"Score: " + NULL` is `"Score: "` |
| conditions | false |
| `==` | equal to `0`, `FALSE` and `NULL`; not equal to `""` |

## The truth rules

`IF`, `WHILE` and every logic operator read values the same way:

| Counts as true | Counts as false | Anything else |
| --- | --- | --- |
| `TRUE`, `1`, `"TRUE"`, `"1"` | `FALSE`, `0`, `NULL`, `"FALSE"`, `"0"` | is an error |

So `IF 2 { }`, `IF "hello" { }`, `IF "" { }` and `IF "true" { }` are all errors.

## Converting

| Command | Gives | Examples |
| --- | --- | --- |
| `NUM(x)` | a number | `NUM("25")` is `25`, `NUM("0xFF")` is `255`, `NUM(TRUE)` is `1`, `NUM(NULL)` is `0` |
| `STR(x)` | text | `STR(25)` is `"25"`, `STR([1, "a"])` is `'[1, "a"]'` |
| `ENG(n)` | engineering notation (exponent a multiple of 3) | `ENG(1500)` is `"1.5e3"`, `ENG(15000)` is `"15e3"` |
| `HEX(n)` | hex text, for whole numbers | `HEX(255)` is `"0xFF"`, `HEX(-26)` is `"-0x1A"` |

Conversions never change the original variable; only assigning the result does.

### Which text counts as a number

Text in these forms counts as a number wherever numeric text is accepted: `NUM()`, `==`, `<`, `>`, `<=`, `>=`, `<==` and `>==`.

| Counts | Doesn't count (stays text) |
| --- | --- |
| `"25"`, `"-25"`, `"0"`, `"2.5"`, `"-0.5"`, `"1e3"`, `"1.5e-3"`, `"0xFF"`, `"-0x1A"` | `"025"`, `" 25 "`, `"1,000"`, `".5"`, `""` |

`NUM()` on text that doesn't count is an error: `can't convert "hello" to a number`.
