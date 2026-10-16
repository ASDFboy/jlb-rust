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

## Choosing the interpreter

A program can say which interpreter runs it, on its first line:

```text
>> @interpreter c
PRINT("this always runs with jlb-c")
```

- **`rust` or `c`.** It's a comment, so it changes nothing else.
- **It wins over everything else:** run with the other interpreter (or double-clicked, or picked in the IDE), the program is handed over to the one it asks for. That interpreter has to be in the same folder as the other one, as it is when they come with the IDE.
- **Only the program's own first line counts.** Files brought in with `IMPORT` and `LOAD` follow the program.
- **Compare in the IDE still runs both,** since that's what it's for.

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

These modes let code editors, like [JLB IDE](https://github.com/ASDFboy/jlb-ide), work with `jlb`. Each one answers in JSON, one line at a time, so any editor can read it. The C interpreter offers the same modes, apart from `--debug`.

| Command | What it does |
| --- | --- |
| `jlb --check file.jlb` | checks the syntax without running anything. Use `-` instead of a file name to send the code through input. |
| `jlb --ide file.jlb` | runs a program for an editor: output is sent the moment it's printed, there's never a pause, and a crash also prints a JSON report |
| `jlb --repl` | starts a Quick-try session that keeps its variables between snippets |
| `jlb --debug file.jlb` | runs a program for an editor's debugger, which can stop it at breakpoints, step through it line by line and look at every variable |

Lines meant for the editor start with the ASCII "record separator" character (code 30), shown here as `␞`.

**`--check`** prints `{"ok":true}`, or the problems it finds. The first problem's `message`, `line` and `col` are at the top, and `errors` lists every problem found, up to 10 (`col` is `null` when the problem is a whole line):

```text
{"ok":false,"message":"!= isn't an operator in .jlb; use !== (or !=== for the strict version)","line":2,"col":7,
  "errors":[{"message":"!= isn't an operator in .jlb; use !== (or !=== for the strict version)","line":2,"col":7},
            {"message":"did you mean PRINT? Built-in commands are written in ALL CAPS","line":3,"col":5}]}
```

(It's one line; it's split here to fit.) To find more than one problem, `--check` sets each broken line aside and checks again. It stops early at a problem that affects the lines after it, like a block that's never closed.

**`--ide`** prints the normal crash report, then a `␞` line with the same report as JSON:

```text
␞{"message":"can't divide by zero","file":"crash.jlb","path":"C:\\code\\crash.jlb","line":3,"traced":true,
  "frames":[{"name":"f","line":3,"path":"C:\\code\\crash.jlb","vars":[{"name":"n","value":"3","type":"NUMBER"}]}],
  "outside":[{"name":"name","value":"\"Alex\"","type":"TEXT"}]}
```

(It's one line; it's split here to fit.) `path` is the full path of the file the error happened in, which can be a file brought in with `IMPORT` or `LOAD`. `frames` lists the running functions, innermost first, each with the path of the file it's in. Each value is written the way `PRINT` shows it inside a table, so text keeps its quotes.

**`--repl`** reads code until a line holding only `␞`, runs it, then prints `␞` and a JSON reply:

| Snippet | Reply |
| --- | --- |
| a single expression, like `x * 2` | `{"ok":true,"value":"10","type":"NUMBER"}` |
| statements, or a command that gives back nothing (like `PRINT`) | `{"ok":true}` |
| `END` | `{"ok":true,"ended":true}` |
| a crash or syntax error | `{"ok":false,"error":{...the --ide report...}}` |

Anything the snippet prints comes before the reply. Sending `␞RESET` clears every variable.

`INPUT` works too: instead of printing its prompt, it prints `␞{"input":"the prompt"}` and reads the answer as the next line of input. The snippet then carries on.


**`--debug`** runs like `--ide`, and the editor can also stop the program and look inside it. It starts by printing `␞{"event":"ready"}`, then waits for the editor to set breakpoints and say `continue` (or `step`, to stop at the first line).

The editor sends commands as input lines starting with `␞`; any other line is an answer for `INPUT`:

| Command | What it does |
| --- | --- |
| `␞break <path><TAB><line>,<line>,...` | sets the breakpoints for one file, replacing its old ones (no lines removes them). Works while the program runs, too. |
| `␞continue` | runs until the next breakpoint |
| `␞step` | stops at the very next statement, going into functions |
| `␞over` | stops at the next statement in this function (or in the code that called it) |
| `␞out` | stops once the current function has finished |
| `␞pause` | stops as soon as possible (useful when a loop never ends) |
| `␞eval <id><TAB><expression>` | works out an expression with the stopped function's variables |
| `␞view <id><TAB><expression>` | the same, but sends much more of a big table (for a table viewer) |
| `␞stop` | ends the program |

When the program stops, `jlb` prints (on standard error, like the crash report) a `␞` line:

```text
␞{"event":"paused","reason":"breakpoint","path":"C:\\code\\game.jlb","line":4,"message":null,
  "stack":[{"name":"heal","path":"C:\\code\\game.jlb","line":4,"vars":[{"name":"amount","type":"NUMBER","value":"1"}]},
           {"name":"","path":"C:\\code\\game.jlb","line":9,"vars":[{"name":"i","type":"NUMBER","value":"1"}]}],
  "globals":[{"name":"hero","type":"TABLE","value":"[name: \"Ann\", hp: 10]","items":[
     {"name":"name","type":"TEXT","value":"\"Ann\""},{"name":"hp","type":"NUMBER","value":"10"}],"more":0}]}
```

`reason` is `breakpoint`, `step`, `pause`, or `error`: an error that no `TRY` will catch stops the program first, with `message` saying what went wrong, so everything can still be looked at. After the editor's next command the program carries on (and for an error, ends with the usual crash report). `stack` lists the running functions, innermost first, then the file's top level (with an empty `name`), each with its own variables; `globals` are the variables outside functions. A table's `items` list its entries (up to 200 each, and a few thousand values in all); `more` counts the ones left out. When the program carries on, `jlb` prints `␞{"event":"resumed"}`.

`eval` (and `view`) replies with `␞{"event":"eval","id":"<id>","ok":true,"value":{...}}`, or `"ok":false` and a `message`. Only expressions can be worked out, and they can't stop at breakpoints, but they can change variables (`x += 1` isn't allowed, but a function that changes a shared table is), so keep them simple.
