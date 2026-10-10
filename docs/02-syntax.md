# 2. Syntax and style

## Names

Case tells you who owns a name: **ALL CAPS is the language, lowercase is yours.**

| Kind | Rule | Examples |
| --- | --- | --- |
| Keywords and built-in commands | ALL CAPS | `PRINT`, `IF`, `FUNC`, `TRUE` |
| Your variables, functions and table names | lowercase letters, digits and `_` | `score`, `player_2`, `add_points` |

These rules are enforced, with a suggested fix:

```text
playerScore = 10     >> error: "playerScore" should be "player_score"
Print("hi")          >> error: did you mean PRINT?
2nd_place = 1        >> error: names can't start with a digit
SCORE = 1            >> error: SCORE isn't a command .jlb knows
```

Names written after `.` or before `:` in a table follow the same rule. To use any text as a name, put it in brackets as text: `player["Full Name"]`.

## Comments

```text
>> a comment runs to the end of the line
score = 10   >> comments can follow code

>>> a multi-line comment starts with >>>
    and ends at the first <<<
```

Comment symbols inside text are just text: `PRINT(">>>")` prints `>>>`. Because `>>` always starts a comment, there's no `>>` operator.

## Statements

A statement ends at the end of a line. To put two statements on one line, separate them with `;`:

```text
score = 10
a = 1; b = 2; PRINT(a + b)
a = 1 b = 2          >> error: two statements on one line need a ; between them
```

A statement continues onto the next line when it clearly isn't finished:

- **Inside unclosed `( )` or `[ ]`,** it runs until every bracket is closed.
- **When a line ends with an operator,** like `+`, `&`, `==` or `,`.

```text
player = [name: "Alex",
          score: 10,
          lives: 3]

IF score > 10 &
   lives > 0 {
    PRINT("ok")
}

total = 1 +
    2 +
    3
```

## Blocks

Blocks are wrapped in curly braces `{ }`. They can span lines or sit on one line:

```text
IF score > 10 {
    PRINT("High")
}
REPEAT 3 { PRINT("Hello") }
```

## Whitespace

- Spaces and tabs between terms are optional, and any amount is fine: `score=10`, `score = 10` and `score    =    10` mean the same thing.
- Indentation has no meaning. Tabs are the convention, but only for readability.
- Spaces can't go inside a name, number, keyword or operator: `==` works but `= =` doesn't, and `!==` works but `! ==` doesn't.

## Keywords

```text
IF  ELIF  ELSEIF  ELSE  WHILE  REPEAT  FOR  IN  TO  STEP  BREAK  SKIP
FUNC  RETURN  LOCAL  TRY  FAIL  GOTO  POINT  IMPORT  LOAD  END
TRUE  FALSE  NULL  UNDEFINE  INSERT  DROP
```

Every built-in command is listed in [Built-in commands](10-built-ins.md).
