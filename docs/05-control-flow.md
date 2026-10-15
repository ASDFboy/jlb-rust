# 5. Control flow

## IF / ELIF / ELSE

```text
IF score > 10 {
    PRINT("High")
} ELIF score == 10 {
    PRINT("Exactly ten")
} ELSE {
    PRINT("Low")
}
```

`ELSE IF` and `ELSEIF` mean exactly the same as `ELIF`. Conditions follow the [truth rules](03-values-and-types.md#the-truth-rules). Writing `IF x = 5` is an error that suggests `==`.

## WHILE

Repeats while the condition is true:

```text
n = 0
WHILE n < 3 { n += 1 }
```

## REPEAT

Runs a block a set number of times:

```text
REPEAT 3 { PRINT("Hello") }
```

`REPEAT 0` does nothing. A negative count (`REPEAT -1`), a fractional count (`REPEAT 2.5`) or text (`REPEAT "3"`) is an error.

## FOR over tables and text

```text
t = ["apple", "pear", name: "Alex"]

FOR value IN t {
    PRINT(value)          >> "apple", "pear", "Alex"
}

FOR key, value IN t {
    PRINT(key, value)     >> 1 apple / 2 pear / name Alex
}

FOR c IN "Alex" { PRINT(c) }        >> A, l, e, x
FOR i, c IN "ab" { PRINT(i, c) }    >> 1 a / 2 b
```

- **With one variable,** you get each value. **With two,** you get the position or name, then the value.
- **Numbered items come first,** in order, then named entries in the order they were added.
- **The loop variables exist only inside the loop.** After the closing `}`, using them is an error.
- **The loop variable is a copy.** To change the real items, share it with `@`:

  ```text
  FOR @enemy IN enemies { enemy.health -= 10 }   >> changes the real enemies
  FOR key, @value IN t { ... }
  ```

## Counting loops

```text
FOR i IN 1 TO 5 { PRINT(i) }              >> 1, 2, 3, 4, 5
FOR i IN 0 TO 10 STEP 2 { PRINT(i) }      >> 0, 2, 4, 6, 8, 10
FOR i IN 5 TO 1 { PRINT(i) }              >> 5, 4, 3, 2, 1
FOR x IN 0 TO 1 STEP 0.25 { PRINT(x) }    >> 0, 0.25, 0.5, 0.75, 1
```

- **Both ends are included.**
- **It counts down automatically** when the start is bigger than the end; `STEP` only sets the size of each jump.
- **`STEP` can be a decimal.** `STEP 0` is an error.

## Loop variables

Loop variables belong to the loop. They don't overwrite an outside variable with the same name, and they're gone after the loop ends.

## BREAK and SKIP

`BREAK` leaves the loop. `SKIP` jumps straight to the next round. Both work in every kind of loop, and using either outside a loop is an error.

```text
FOR i IN 1 TO 10 {
    IF i == 3 { SKIP }
    IF i == 6 { BREAK }
    PRINT(i)              >> 1, 2, 4, 5
}
```

## GOTO and POINT

`POINT name` marks a spot in the code, and `GOTO name` jumps there, forward or backward (like TI-BASIC's `Lbl` and `Goto`).

```text
tries = 0
POINT retry
tries += 1
IF tries < 3 {
    GOTO retry
}
PRINT("Done after " + tries + " tries")   >> Done after 3 tries
```

- **Point names follow the lowercase naming rules.** Each name can only be used once per function, or once at a file's top level.
- **`GOTO` can jump out of an `IF` block or a loop,** which leaves it cleanly, like `BREAK`. It can't jump into a block.
- **`GOTO` stays inside its own function:** it can't jump into or out of a function.
- **A duplicate `POINT` is an error,** and so is a `GOTO` with no reachable `POINT`.

## END

`END` stops the whole program immediately, from anywhere, including inside functions and loops. It isn't an error: no report is printed, and `TRY` doesn't catch it.

```text
IF lives == 0 {
    PRINT("Game over")
    END
}
```
