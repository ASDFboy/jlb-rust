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

## Editor modes

These modes let code editors, like [JLB IDE](https://github.com/ASDFboy/jlb-ide), work with `jlb`. Each one answers in JSON, one line at a time, so any editor can read it. The C interpreter offers the same modes.

| Command | What it does |
| --- | --- |
| `jlb --check file.jlb` | checks the syntax without running anything. Use `-` instead of a file name to send the code through input. |
| `jlb --ide file.jlb` | runs a program for an editor: output is sent the moment it's printed, there's never a pause, and a crash also prints a JSON report |
| `jlb --repl` | starts a Quick-try session that keeps its variables between snippets |

Lines meant for the editor start with the ASCII "record separator" character (code 30), shown here as `␞`.

**`--check`** prints `{"ok":true}`, or the first problem it finds, with its line and column (`col` is `null` when the problem is a whole line):

```text
{"ok":false,"message":"!= isn't an operator in .jlb; use !== (or !=== for the strict version)","line":2,"col":7}
```

**`--ide`** prints the normal crash report, then a `␞` line with the same report as JSON:

```text
␞{"message":"can't divide by zero","file":"crash.jlb","line":3,"traced":true,
  "frames":[{"name":"f","line":3,"vars":[{"name":"n","value":"3","type":"NUMBER"}]}],
  "outside":[{"name":"name","value":"\"Alex\"","type":"TEXT"}]}
```

(It's one line; it's split here to fit.) `frames` lists the running functions, innermost first. Each value is written the way `PRINT` shows it inside a table, so text keeps its quotes.

**`--repl`** reads code until a line holding only `␞`, runs it, then prints `␞` and a JSON reply:

| Snippet | Reply |
| --- | --- |
| a single expression, like `x * 2` | `{"ok":true,"value":"10","type":"NUMBER"}` |
| statements, or a command that gives back nothing (like `PRINT`) | `{"ok":true}` |
| `END` | `{"ok":true,"ended":true}` |
| a crash or syntax error | `{"ok":false,"error":{...the --ide report...}}` |

Anything the snippet prints comes before the reply. Sending `␞RESET` clears every variable.

