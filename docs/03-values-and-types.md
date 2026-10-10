# 3. Values and types

Variables need no declaration. A variable is created when it's first assigned, and it can hold any type at any time:

```text
value = 10
value = "hello"      >> fine: the type can change
```

`TYPE(x)` tells you what something is:

| Type | Examples | `TYPE` gives |
| --- | --- | --- |
| number | `10`, `2.5`, `-3`, `1e3`, `0xFF` | `"NUMBER"` |
| text | `"hello"`, `'it\'s'` | `"TEXT"` |
| true/false | `TRUE`, `FALSE` | `"BOOL"` |
| nothing | `NULL` | `"NULL"` |
| table | `[1, 2]`, `[name: "Alex"]` | `"TABLE"` |
| function | `FUNC(x) { RETURN x }`, `PRINT` | `"FUNC"` |

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

Under the hood, numbers are 64-bit floating point. Whole numbers are exact up to about 9 quadrillion, and some decimals can't be stored exactly: `0.1 + 0.2` shows `0.30000000000000004`. Use `ROUND(x, 2)` to tidy results.

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
