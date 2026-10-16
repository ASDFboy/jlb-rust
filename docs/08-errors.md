# 8. Errors

## Crash reports

An uncaught error stops the program and prints a report with:

- the line;
- an explanation;
- the chain of function calls that led there;
- every variable's value at that moment.

```text
ERROR in main.jlb, line 5: can't convert "hello" to a number

Called from:
    load_save()   line 5
    start_game()  line 9

Variables:
    load_save:  file = "save.txt", data = "hello"
    start_game: tries = 1
    outside:    score = 10, lives = 3
```

The report goes to the error output, and `jlb` exits with code 1.

## Syntax errors

Mistakes in how the code is written are reported before anything runs, with the line and a suggestion where possible:

```text
ERROR in main.jlb, line 2: = assigns a value; use == to compare
ERROR in main.jlb, line 1: "playerScore" should be "player_score" (your own names are lowercase with underscores)
ERROR in main.jlb, line 1: did you mean PRINT? Built-in commands are written in ALL CAPS
ERROR in main.jlb, line 1: two statements on one line need a ; between them
ERROR in main.jlb, line 1: != isn't an operator in .jlb; use !== (or !=== for the strict version)
```

## TRY / FAIL

`TRY` runs a block; if an error happens inside it, the `FAIL` block runs instead of the program crashing:

```text
TRY {
    x = NUM("hello")
} FAIL err {
    PRINT(err.message)     >> can't convert "hello" to a number
    PRINT(err.line)        >> 2
}
PRINT("still running")
```

`err` is a table:

| Entry | What it holds |
| --- | --- |
| `err.message` | the explanation |
| `err.line` | the line it happened on |
| `err.file` | the file it happened in |
| `err.variables` | a table of every variable's value when it failed |

`PRINT(err)` shows the full report, the same as a crash would. `err` only exists inside the `FAIL` block.

## ERROR

`ERROR("message")` raises your own error. It stops the program like a built-in error, or gets caught by `TRY`:

```text
DEF risky(n) {
    IF n > 5 { ERROR("too big: " + n) }
    RETURN n
}
TRY { risky(9) } FAIL e { PRINT(e.message) }   >> too big: 9
```

## END isn't an error

`END` stops the program without a report, and `TRY` doesn't catch it.
