# 1. Getting started

## Your first program

Make a file called `hello.jlb` containing:

```text
PRINT("Hello, world!")
```

Then run it:

- **Windows, after running `windows\install.bat`:** double-click `hello.jlb`.
- **Any system, from a terminal:** `jlb hello.jlb` (or `jlb.exe hello.jlb`).

A program runs from top to bottom. When it's double-clicked on Windows, the window waits for you to press Enter before closing, so you can read the output. To get that pause from a terminal too, use `jlb --pause hello.jlb`.

## A slightly bigger program

```text
>> Ask for a name and say hello
name = INPUT("What's your name? ")
PRINT("Nice to meet you, " + name + "!")

fruits = ["apple", "pear", "orange"]
FOR i, fruit IN fruits {
    PRINT(i + ". " + fruit + " backwards is " + fruit * -1)
}
```

Output, after typing `Alex`:

```text
What's your name? Alex
Nice to meet you, Alex!
1. apple backwards is elppa
2. pear backwards is raep
3. orange backwards is egnaro
```

A few things to notice:

- **Built-in commands are ALL CAPS** (`PRINT`, `INPUT`, `FOR`). Your own names are lowercase (`name`, `fruits`).
- **`>>` starts a comment.**
- **`+` with any text joins things together,** so `i + ". "` works even though `i` is a number.
- **Text can be multiplied:** `* -1` reverses it.
- **Tables (`[...]`) start at position 1.**

## The `jlb` command

| Command | What it does |
| --- | --- |
| `jlb file.jlb` | runs a program |
| `jlb --pause file.jlb` | runs a program, then waits for Enter |
| `jlb --version` | shows the interpreter version |
| `jlb --check file.jlb` | checks the syntax without running anything ([editor modes](09-programs-and-files.md#editor-modes)) |

When a program crashes, `jlb` prints an error report and exits with code 1. A syntax error, or a file it can't find, also exits with a non-zero code.
