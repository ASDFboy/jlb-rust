# .jlb language reference

This is the complete reference for the .jlb language, as run by this interpreter.

1. [Getting started](01-getting-started.md): your first program, and how to run it
2. [Syntax and style](02-syntax.md): names, comments, statements, blocks, whitespace
3. [Values and types](03-values-and-types.md): numbers, text, TRUE/FALSE, NULL, conversions
4. [Operators](04-operators.md): math, text and table operators, comparisons, logic, order of operations
5. [Control flow](05-control-flow.md): IF, WHILE, REPEAT, FOR, BREAK/SKIP, GOTO/POINT, END
6. [Functions](06-functions.md): DEF, scope, LOCAL, GLOBAL, inputs, functions as values
7. [Tables and text](07-tables-and-text.md): positions, names, slices, copying and sharing
8. [Errors](08-errors.md): crash reports, TRY/FAIL, ERROR
9. [Running programs, files and modules](09-programs-and-files.md): running order, IMPORT, LOAD, UNDEFINE
10. [Built-in commands](10-built-ins.md): every ALL-CAPS command
11. [Tasks](11-tasks.md): START, AWAIT, DONE, CANCEL: things happening at the same time

## Quick reference

```text
>> comment                         >>> multi-line comment <<<
x = 10                             copy a value into x
y @= x                             share x (tables and text)
"raw text"   'text with \n codes'  [1, 2, name: "Alex"]
TRUE  FALSE  NULL

+ - * / ^ !^ % !%                  math (also text and table tricks)
== === !== !===                    loose and strict equality
< > <= >= <== >==                  comparisons
& && !& !&&  | || !| !||  ~ !~  !  logic

IF c { } ELIF c { } ELSE { }
WHILE c { }      REPEAT n { }
FOR x IN t { }   FOR k, v IN t { }   FOR i IN 1 TO 10 STEP 2 { }
BREAK  SKIP  GOTO name  POINT name  END

DEF name(a, b = 1, @shared, ...rest) { RETURN a }
LOCAL x = 0      name.x            DEF(x) { RETURN x * 2 }
CONST max = 10                     a name that can never change
GLOBAL x = 1                       in a function: the outside x (names are local there otherwise)
t = START f(x)    AWAIT(t)         run f(x) side by side, then wait for its result
TRY { } FAIL err { }               ERROR("message")
IMPORT "file.jlb"                  LOAD "file.jlb"
UNDEFINE(x)
```
