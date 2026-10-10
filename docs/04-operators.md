# 4. Operators

## Assignment

| Operator | Meaning |
| --- | --- |
| `x = value` | copies the value into `x` |
| `x @= y` | makes `x` share `y` (see [copying and sharing](07-tables-and-text.md#copying-and-sharing)) |
| `x += v`, `-=`, `*=`, `/=`, `^=`, `!^=`, `%=`, `!%=` | `x = x + v` and so on, with exactly the same rules as the operator |

An assignment is always a statement by itself; it can't be used inside an expression.

## Math

| Operator | Meaning | Example |
| --- | --- | --- |
| `+` `-` `*` `/` | the usual | `5 / 2` is `2.5` |
| `^` | power | `2 ^ 3` is `8` |
| `!^` | root: left value inside the radical, right value is the root | `9 !^ 2` is `3`, `27 !^ 3` is `3` |
| `%` | remainder of long division | `7 % 3` is `1` |
| `!%` | answer of long division (the whole-number part) | `7 !% 3` is `2` |
| `-x` | negative | `-5` |

- Dividing by zero (`/`, `%` or `!%`) is an error.
- Math is exact, with no rounding errors ([how numbers work](03-values-and-types.md#numbers-are-exact)): `0.1 + 0.2` is `0.3`, and `0.3 % 0.1` is `0`.
- Roots and fractional powers are exact whenever the answer comes out even: `0.25 !^ 2` is `0.5`, and `8 ^ (2 / 3)` is `4`. Otherwise they're approximate: `2 !^ 2` is `1.4142135623730951`.
- `%` and `!%` work on decimals too: `7.5 % 2` is `1.5`, and `7.5 !% 2` is `3`. With negatives, `!%` cuts off the decimal like long division by hand: `-7 !% 2` is `-3`, and `-7 % 2` is `-1`.
- An even root of a negative number is an error: `(-9) !^ 2`. An odd root works: `(-27) !^ 3` is `-3`. The parentheses matter, because `!^` happens before the minus sign: `-9 !^ 2` means `-(9 !^ 2)`, which is `-3`.
- `NULL` counts as `0` in math.

## Text operators

Text is always treated as text, even when it looks like a number.

| Expression | Result | Rule |
| --- | --- | --- |
| `"25" + 2`, `2 + "25"` | `"252"`, `"225"` | `+` with any text joins |
| `"25" * 2`, `2 * "25"` | `"2525"` | text repeats, either order |
| `"abc" * -1` | `"cba"` | negative reverses |
| `"abc" * -2` | `"cbacba"` | reverses, then repeats |
| `"hi" * 0` | `""` | empty text |
| `"hi" * 2.5` | error | can't repeat a fraction of times |
| `"25" * "2"` | error | the repeat count must be a number |
| `"hello" - 2` | `"hel"` | trims characters from the end |
| `"hello" - 20` | `""` | trims everything |
| `"hello" - -2` | `"hello2"` | minus a negative becomes `+` |
| `"hello world" - "o"` | `"hell wrld"` | text minus text removes every copy |
| `2 - "25"` | error | a number minus text has no meaning |
| `"25" / 2` | error | no `/`, `^`, `!^`, `%` or `!%` on text |

## Table operators

`*` and `-` work on tables the same way as on text. They affect the numbered items; named entries are kept as they are.

| Expression | Result | Rule |
| --- | --- | --- |
| `[1, 2] * 2` | `[1, 2, 1, 2]` | repeats |
| `[1, 2, 3] * -1` | `[3, 2, 1]` | reverses |
| `[1, 2, 3] * -2` | `[3, 2, 1, 3, 2, 1]` | reverses, then repeats |
| `[1, 2] * 0` | `[]` | empty |
| `[1, 2, 3, 4] - 2` | `[1, 2]` | removes items from the end |
| `[1, 2] - -2` | `[1, 2, 2]` | minus a negative becomes `+` |
| `["a", "b", "a"] - "a"` | `["b"]` | removes every copy |
| `t + v` | `v` added to the end | |
| `v + t` | `v` added to the start | |
| `[1, 2] + [3, 4]` | `[[1, 2], [3, 4]]` | two tables make a table of both |

## Comparisons

| Operator | Meaning |
| --- | --- |
| `==` | equal, converting numeric text when compared with a number |
| `===` | equal in both value and type, with no conversion |
| `!==` | opposite of `==` |
| `!===` | opposite of `===` |
| `<` `>` | less than, greater than |
| `<=` `>=` | less or equal, greater or equal |
| `<==` `>==` | less than or `==`, greater than or `==` |

`!=` is an error; use `!==`.

```text
"25" == 25      >> TRUE   (numeric text converts)
"025" == 25     >> FALSE  ("025" isn't a valid number)
"25" === 25     >> FALSE  (different types)
"025" == "25"   >> FALSE  (text against text compares the characters)
TRUE == 1       >> TRUE
NULL == 0       >> TRUE
"TRUE" == TRUE  >> TRUE
```

**Tables:** `==` checks whether the contents match. `===` checks whether both names are literally the same shared table (from `@=`).

**Ordering (`<`, `>`, and the rest):**

- **Numbers** compare normally.
- **Text against a number:** the text is converted if it's a valid number (`"25" > 24` is TRUE); otherwise it's an error.
- **Text against text compares lengths:**
  - `"abc" > "ab"` is TRUE.
  - `"5" > "3"` is FALSE, because both are one character long.
  - `"abc" <= "xyz"` is TRUE, because they're the same length.
  - `"abc" <== "xyz"` is FALSE: the same length, but not the same text.
  - For alphabetical order, use `ALPHA_SORT`.
- **For numbers,** `<==` and `>==` act just like `<=` and `>=`.

## Logic

Every logic operator reads its values with the [truth rules](03-values-and-types.md#the-truth-rules) and gives `TRUE` or `FALSE`.

| Operator | Meaning | Checks both sides? |
| --- | --- | --- |
| `!x` | NOT | |
| `&` | AND | only if needed |
| `&&` | AND | always |
| `!&` | NOT AND (true unless both are true) | only if needed |
| `!&&` | NOT AND | always |
| `\|` | OR | only if needed |
| `\|\|` | OR | always |
| `!\|` | NOT OR (true only if both are false) | only if needed |
| `!\|\|` | NOT OR | always |
| `~` | XOR (true if exactly one is true) | always |
| `!~` | XNOR (true if both match) | always |

One symbol checks one side when it can; two symbols always check two sides. The single-symbol versions make safe checks possible:

```text
IF enemy !== NULL & enemy.health > 0 {   >> safe even when there's no enemy
    attack(enemy)
}
IF ready && log_check() { }              >> log_check() runs every time
```

This is the reverse of C and JavaScript, where `&&` is the one that stops early.

## Order of operations

From first to last. Operators in the same row run left to right, except `^` and `!^`, which run right to left (`2 ^ 3 ^ 2` is `2 ^ 9`). Parentheses always win.

| Rank | Operators |
| --- | --- |
| 1 (first) | `( )`, function calls, `.` and `[ ]` |
| 2 | `^`, `!^` |
| 3 | `-x`, `!x` |
| 4 | `*`, `/`, `%`, `!%` |
| 5 | `+`, `-` |
| 6 | `<`, `>`, `<=`, `>=`, `<==`, `>==` |
| 7 | `==`, `===`, `!==`, `!===` |
| 8 | `&`, `&&`, `!&`, `!&&` |
| 9 | `~`, `!~` |
| 10 (last) | `\|`, `\|\|`, `!\|`, `!\|\|` |

```text
2 + 3 * 4                  >> 14
-2 ^ 2                     >> -4
score > 10 & lives > 0     >> comparisons run before &
a | b & c                  >> a | (b & c)
```
