<img src="docs/images/jlb-logo.svg" width="112" alt="The JLB logo">

# jlb-rust

The interpreter for **.jlb**, a small programming language, written in Rust.

**Authors:** [ASDFboy](https://github.com/ASDFboy) and Claude

```text
>> guess.jlb: a number guessing game
secret = RANDOM(1, 100)
tries = 0
PRINT("I'm thinking of a number from 1 to 100.")

POINT ask
guess = INPUT("Your guess: ")
TRY {
    guess = NUM(guess)
} FAIL err {
    PRINT("That's not a number, try again.")
    GOTO ask
}
tries += 1

IF guess < secret {
    PRINT("Higher!")
    GOTO ask
} ELIF guess > secret {
    PRINT("Lower!")
    GOTO ask
}
PRINT("You got it in " + tries + " tries!")
```

## What is .jlb?

.jlb is a simple, general-purpose scripting language. Its name comes from its three main influences:

- **JavaScript**: variables can hold anything, and types change automatically when it makes sense to.
- **Lua**: tables are the one collection type, holding both numbered lists and named entries.
- **BASIC**: built-in commands are written in ALL CAPS, so you can always tell the language's words from your own.

It also has a lot of its own ideas. Text and tables can be multiplied, subtracted and reversed (`"abc" * -1` is `"cba"`). A whole family of `!` operators covers things like NAND, roots and long division. And when something goes wrong, the crash report shows every variable's value.

There's also a C interpreter, [jlb-c](https://github.com/ASDFboy/jlb-c). Both must pass the same test programs in [`tests/cases`](tests/cases).

To write .jlb, there's [**JLB IDE**](https://github.com/ASDFboy/jlb-ide): an editor with live mistake checking, a step-by-step debugger, a crash report viewer, hover explanations of .jlb's operators, and a Quick-try console. It comes with this interpreter.

## Pros and cons

| Pros | Cons |
| --- | --- |
| **Easy to read.** ALL CAPS commands and lowercase names never get mixed up, and naming mistakes come with a suggested fix. | **Brand new.** There are no libraries, package manager or community yet, and [JLB IDE](https://github.com/ASDFboy/jlb-ide) is the only editor that understands .jlb. |
| **Flexible like JavaScript.** Variables can switch types, and `==` converts while `===` doesn't. | **No graphics, sound, networking or live keyboard input yet.** Programs talk through `PRINT` and `INPUT`. |
| **One simple collection type.** Tables work as lists, as records with named entries, or both at once. | **Slower than compiled languages.** It runs your code directly instead of compiling it, so heavy number crunching is much slower than in C or Rust. |
| **Powerful text and table tricks.** You get repeat, reverse, trim, slices (`t[2 TO 4]`) and negative positions (`t[-1]`). | **Some rules are reversed from other languages.** `&` stops early but `&&` checks both sides, `!==` is the loose not-equal, and single quotes are the ones with backslash codes. |
| **Safe by default.** `=` copies tables, so nothing changes behind your back, while `@` shares on purpose. Copies are cheap thanks to copy-on-write. | **Comparing text with `<` and `>` compares lengths,** not alphabetical order (`ALPHA_SORT` handles alphabetical). |
| **Very helpful errors.** A crash shows the line, the chain of function calls, and every variable's value. | **No closures.** A function can't see the locals of a function it's written inside. |
| **Exact math.** `0.1 + 0.2` is exactly `0.3`, `1 / 3 * 3` is exactly `1`, and numbers grow automatically to as many digits as they need (`2 ^ 100` keeps all 31 digits). | **Some answers are still approximate.** Roots that don't come out even (`2 !^ 2`), trig in radians and `PI()` have about 16 significant digits, because their digits go on forever without repeating. |
| **Small and fast to start.** It's one ~750 KB program with no dependencies, and a million-step loop takes about 0.2 seconds. | **Exactness costs some speed with decimals.** Whole-number math runs at full speed, but long loops of decimal or fraction math are slower than plain floating point would be. |
| **Double-click to run on Windows,** with the window staying open so you can read the output. | **Double-click setup is Windows only.** On macOS and Linux you run programs from a terminal. |

## Setting it up

### Windows (quickest)

1. Download this repo: click the green **Code** button, then **Download ZIP**, and unzip it somewhere permanent, like `Documents\jlb`.
2. Open the `windows` folder and double-click **`install.bat`**. It needs no admin rights and only affects your own Windows account.
3. Double-click any `.jlb` file, such as `examples\hello.jlb`. A window opens, runs the program, and waits for you to press Enter before closing.

To edit a program, right-click it and choose **Edit** (it opens in Notepad), or use any text editor.
To undo the setup, double-click `windows\uninstall.bat`.
If you move the folder later, run `install.bat` again.

> If double-clicking opens a different app, Windows remembered an earlier choice. Right-click a `.jlb` file, choose **Open with → Choose another app**, browse to `windows\jlb.exe`, and tick **Always**.

### Running from a terminal

```text
jlb.exe examples\hello.jlb
```

To type just `jlb` from any folder, add the `windows` folder to your PATH. Search Windows for "Edit environment variables for your account", choose **Path → Edit → New**, and paste the folder's location.

### Building from source (Windows, macOS, Linux)

1. Install Rust (free) from <https://rustup.rs>.
2. Build it:

   ```text
   cargo build --release
   ```

   The program appears at `target/release/jlb` (`jlb.exe` on Windows). On Windows, `windows\install.bat` also finds it there automatically.
3. Run the tests:

   ```text
   cargo test --release
   ```

There are no dependencies beyond Rust's standard library.

## Documentation

The full language reference is in [**docs/**](docs/README.md):

1. [Getting started](docs/01-getting-started.md)
2. [Syntax and style](docs/02-syntax.md)
3. [Values and types](docs/03-values-and-types.md)
4. [Operators](docs/04-operators.md)
5. [Control flow](docs/05-control-flow.md)
6. [Functions](docs/06-functions.md)
7. [Tables and text](docs/07-tables-and-text.md)
8. [Errors](docs/08-errors.md)
9. [Running programs, files and modules](docs/09-programs-and-files.md)
10. [Built-in commands](docs/10-built-ins.md)

## Project layout

| Path | What's in it |
| --- | --- |
| `src/lexer.rs` | turns source text into tokens: names, numbers, both kinds of quotes, comments |
| `src/parser.rs` | builds the program's structure and reports syntax errors |
| `src/interp.rs` | runs the program: variables, scope, loops, functions, GOTO, errors, IMPORT/LOAD |
| `src/ops.rs` | every operator: math, text and table tricks, comparisons, truth rules |
| `src/builtins.rs` | the ALL-CAPS built-in commands |
| `src/value.rs` | values, tables, copy-on-write, crash reports, number formatting |
| `src/main.rs` | the `jlb` command, including the double-click pause on Windows |
| `tests/cases/` | the shared test programs (`.jlb`), with expected output (`.out`) and typed input (`.in`) |
| `windows/` | `install.bat`, `uninstall.bat` and a prebuilt `jlb.exe` |
| `examples/` | small programs to try |

## Tests

`cargo test --release` runs every program in `tests/cases/` and compares everything it prints, including error reports, with the matching `.out` file. To add a test, write `name.jlb` plus `name.out` with the exact expected output, and `name.in` if it uses `INPUT`.

## Authors

Made by **ASDFboy** and **Claude**.
