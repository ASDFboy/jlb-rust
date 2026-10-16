# 7. Tables and text

Tables are the only collection type. A table can hold numbered items, named entries, or both:

```text
fruits = ["apple", "pear", "orange"]
player = [name: "Alex", score: 10, greet: say_hi]
mixed = ["apple", name: "Alex", 42]       >> ["apple", 42, name: "Alex"]
```

Names are written `name: value`. To use any text as a name, put it in quotes: `["Full Name": "Alex K"]`.

## Reading

```text
fruits[1]             >> "apple"   (positions start at 1)
fruits[-1]            >> "orange"  (negative positions count from the end)
fruits[0]             >> NULL
fruits[10]            >> NULL      (reading never errors)

player.name           >> "Alex"
player["name"]        >> "Alex"    (the same thing)
key = "score"
player[key]           >> 10        (brackets take a name stored in a variable)
player.lives          >> NULL
player.greet()        >> calls the function stored in the table
```

## Changing

```text
t = ["a", "b", "c"]
t[2] = "x"            >> replaces: ["a", "x", "c"]
t[-1] = "z"           >> replaces the last: ["a", "x", "z"]
t[4] = "d"            >> position LEN(t) + 1 adds to the end
t[10] = "q"           >> error: past the end (tables never have gaps)
t[0] = "q"            >> error

player.lives = 3      >> assigning a new name adds it
player.score = NULL   >> keeps the entry; it just holds NULL
UNDEFINE(player.score)   >> removes the entry completely
```

### Adding and removing

| Code | Result |
| --- | --- |
| `t + v` / `t += v` | adds `v` to the end |
| `v + t` | adds `v` to the start |
| `[1, 2] + [3, 4]` | `[[1, 2], [3, 4]]` (a table holding both tables) |
| `INSERT(t, v, pos)` | puts `v` so it ends up at `pos`; later items shift up |
| `DROP(t, pos)` | removes the item at `pos`; later items shift down |

```text
fruits = ["apple", "pear"]
fruits += "orange"              >> ["apple", "pear", "orange"]
INSERT(fruits, "kiwi", 1)       >> ["kiwi", "apple", "pear", "orange"]
DROP(fruits, 2)                 >> ["kiwi", "pear", "orange"]
DROP(fruits, -1)                >> ["kiwi", "pear"]
INSERT(fruits, "fig", -1)       >> ["kiwi", "pear", "fig"]   (fig is now fruits[-1])
INSERT(fruits, "lime", -2)      >> ["kiwi", "pear", "lime", "fig"]
```

- **For `INSERT`,** position `LEN(t) + 1` also adds to the end. To add a table as one item at the end, use `INSERT(t, other, -1)`.
- **For `DROP`,** a position that doesn't exist, including `0`, is an error.
- **Both change what you give them, and also give back the result,** so `x = INSERT("Ale", "x", -1)` works too.

`LEN(t)` counts the numbered items.

### Table math

`*` and `-` work on tables the same way as on text. See [table operators](04-operators.md#table-operators).

```text
[1, 2] * 2            >> [1, 2, 1, 2]
[1, 2, 3] * -1        >> [3, 2, 1]
[1, 2, 3, 4] - 2      >> [1, 2]
["a", "b", "a"] - "a" >> ["b"]
```

## Slices

`[start TO end]` gives part of a table or text, with an optional `STEP`:

```text
name = "Alexander"
name[1 TO 4]          >> "Alex"
name[-3 TO -1]        >> "der"
name[4 TO 1]          >> "xelA"   (start bigger than end: reversed)
"Alex"[2 TO 10]       >> "lex"    (positions past the end are left out)

t = [10, 20, 30, 40, 50]
t[2 TO 4]             >> [20, 30, 40]
t[1 TO 5 STEP 2]      >> [10, 30, 50]
```

Both ends are included, and a slice is always a copy.

## Text positions

Text works like a table of characters, with the same position rules:

```text
name = "Alex"
name[1]               >> "A"
name[-1]              >> "x"
name[10]              >> NULL

name[1] = "G"         >> "Glex"   (replaces one character)
name[1] = "Gr"        >> "Grlex"  (any length: several characters insert)
name[-1] = ""         >> "Grle"   ("" deletes)
name[10] = "x"        >> error: past the end

INSERT("Ale", "x", -1)   >> "Alex"
DROP("Alex", 1)          >> "lex"
FOR c IN "Alex" { }      >> loops over each character
```

`LEN("Alex")` is `4`.

## Copying and sharing

`=` copies tables and text. The copy is complete: tables inside a table are copied too, so nothing stays secretly shared.

`@=` shares instead. Both names then point at the same table or text, so a change through one shows up in the other:

```text
a = [1, 2, 3]
b = a          >> copy: changing b leaves a alone
c @= a         >> shared: c and a are the same table
c += 4         >> a is now [1, 2, 3, 4]; b is still [1, 2, 3]
```

- **The same rule applies to function inputs** (`DEF f(@p)`) and `FOR` loop variables (`FOR @x IN t`).
- **Numbers, TRUE/FALSE and NULL can't be changed in place,** so `@=` acts just like `=` for them.
- **Copies are cheap.** The real copy only happens when one side changes it (copy-on-write), so `b = a` on a huge table costs nothing until you modify `b`.
- **`a == b` checks whether two tables have the same contents.** `a === b` checks whether they're the same shared table.

## Printing tables

`PRINT` shows a table written the way you'd type it, and `STR(t)` gives the same text:

```text
t = ["apple", "pear", name: "Alex", greet: say_hi, inv: ["sword", "shield"]]
PRINT(t)
>> ["apple", "pear", name: "Alex", greet: DEF say_hi, inv: ["sword", "shield"]]
```

Numbered items come first, then named entries. Text keeps its quotes, so `"5"` and `5` look different.

## Helpers

These built-ins work on tables (see [Built-in commands](10-built-ins.md)):

| Command | What it does |
| --- | --- |
| `LEN` | counts the numbered items |
| `KEYS` | lists the positions and names |
| `SORT` / `ALPHA_SORT` | sorted copies |
| `FIND` | the position, name or path of a value |
| `TABLE_MIN` / `TABLE_MAX` | the smallest or largest value |
| `JOIN` | the items joined into text |
| `SPLIT` | text split into a table |
