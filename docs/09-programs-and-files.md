# 9. Running programs, files and modules

## Running order

- **A `.jlb` file runs from top to bottom.** There's no `main()` to write.
- **Everything is defined in order as the file runs.** Calling a function before its `FUNC` line has run is an error, and so is using a variable before it has a value.
- **The check happens when a call actually runs,** so a function can call another function defined below it, as long as both exist by the time the call happens.

```text
greet()                      >> error: greet isn't defined
FUNC greet() { PRINT("hi") }
greet()                      >> "hi"

FUNC a() { RETURN b() }      >> fine: b only has to exist when a runs
FUNC b() { RETURN "b" }
a()
```

## UNDEFINE

`UNDEFINE(name)` removes anything with a name: a variable, a function, a whole table, or a single table entry. Using the name afterwards is an error until it's given a value again.

```text
score = 10
UNDEFINE(score)
PRINT(score)                 >> error: score isn't defined

UNDEFINE(player.score)       >> removes that entry from the table
```

`UNDEFINE` takes the name itself, not its value. If two names share a table through `@=`, it removes only the name you give it.

## IMPORT and LOAD

Both run another `.jlb` file. They differ in how you reach what it defines:

| Statement | How you call `greet` from helpers.jlb |
| --- | --- |
| `IMPORT "helpers.jlb"` | `greet()` |
| `LOAD "helpers.jlb"` | `helpers.greet()` |

- **Paths are relative to the file doing the importing.**
- **A file only runs once,** the first time it's imported or loaded, even if several files import it.
- **If two imported files define the same name, the last import wins.**
- **`LOAD` names its table after the file:** lowercase, with dashes and spaces turned into underscores. `LOAD "My-Helpers.jlb"` gives `my_helpers`.
- **`LOAD` gives a table of the file's values as they were when it loaded.** Its functions keep using that file's own outside variables.

## Files

| Command | What it does |
| --- | --- |
| `READ("save.txt")` | the whole file as text |
| `READ_LINES("save.txt")` | a table with one item per line |
| `WRITE("save.txt", value)` | replaces the file's contents, creating the file if it doesn't exist |
| `EXISTS("save.txt")` | TRUE or FALSE |

File paths are relative to the folder `jlb` is run from. When a program is double-clicked on Windows, that's the program's own folder.

```text
WRITE("save.txt", 'score=10\nlives=3')
lines = READ_LINES("save.txt")      >> ["score=10", "lives=3"]
```

## Input

`INPUT("prompt")` shows the prompt, waits for the user to type a line, and gives back what they typed. It's always text, so use `NUM()` for numbers:

```text
age = INPUT("Age? ")
PRINT(age + 1)          >> "251" if they typed 25
PRINT(NUM(age) + 1)     >> 26
```
