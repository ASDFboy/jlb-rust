# 6. Functions

## Defining and calling

```text
DEF add(a, b) {
    RETURN a + b
}
PRINT(add(2, 3))      >> 5
```

- **`DEF` makes a function.** `FUNC`, its older name, still works the same way.
- **A function without `RETURN` gives `NULL`.**
- **`RETURN` can only be used inside a function.**
- **Functions are defined in order as the file runs,** so calling one before its `DEF` line has run is an error. A function can still call another function defined further down, as long as that one exists by the time the call actually happens.
- **A function can call itself.** About 2,500 calls deep, it stops with an error instead of crashing.

## Scope: which variables a function sees

A function sees its own local variables and the file's outside variables:

- **A function can read the file's outside variables.**
- **A name given a value inside a function is the function's own** (a local), even if an outside variable has the same name. It disappears when the function finishes. The same goes for functions made inside a function.
- **`GLOBAL` before the name** gives the value to the outside name instead.

```text
score = 10
DEF add_points(amount) {
    PRINT(score)             >> 10: outside names can be read
    bonus = amount * 2       >> local
    GLOBAL score += amount   >> changes the outside score
    RETURN bonus
}
```

### GLOBAL

`GLOBAL` works with every way of giving a value, and with `DEF` and `CONST`:

```text
DEF setup() {
    GLOBAL lives = 3                 >> a new outside name (or changes it)
    GLOBAL lives -= 1
    GLOBAL pool @= [1, 2]            >> shared, like @=
    GLOBAL DEF reset() { ... }      >> a function the whole file can use (DEF GLOBAL reset() works too)
    GLOBAL CONST max_hp = 100        >> an outside constant
}
```

- **Without `GLOBAL`, `x += 1` inside a function needs `x` to be the function's own.** Changing an outside `x` that way is an error that says to write `GLOBAL x += 1`. (`x = x + 1` makes a local `x` from the outside one.)
- **The entries of an outside table can be changed directly:** `hero.hp -= 5` changes the outside `hero` when the function has no `hero` of its own.
- **`GLOBAL` goes before a name,** not a table entry, and outside functions it changes nothing.

### LOCAL

Names given a value in a function are already local, so `LOCAL` is only needed to make one before it has a value (`LOCAL x` starts as `NULL`), or to say so clearly:

```text
count = 3
DEF count_enemies() {
    LOCAL count = 0          >> separate from the outside count
    FOR e IN enemies { count += 1 }
    RETURN count
}
>> outside, count is still 3
```

`LOCAL` can only be used inside a function.

### What counts as local

A function's locals are:

- every name given a value inside it without `GLOBAL` (including with `LOCAL`, `CONST` and `DEF`);
- its inputs.

A `FOR` loop's variables (and a `FAIL` block's error name) are different: they exist only inside that loop's `{ }` block, in or out of a function.

### Reaching another function's locals

Inside its own function, a local is used by its short name. Everywhere else, put the function's name in front:

```text
DEF outer() {
    LOCAL secret = 42
    RETURN inner()
}
DEF inner() {
    RETURN outer.secret      >> 42, because outer is still running
}
```

- **This only works while that function is running;** locals disappear when it finishes.
- **If a function is running more than once** (it called itself), `name.var` means the most recent call.
- **A function can't see the locals of a function it's written inside** (there are no closures).

## Inputs

### Optional inputs

Give an input a default with `=`:

```text
DEF greet(name, greeting = "Hi") {
    RETURN greeting + ", " + name
}
greet("Alex")              >> "Hi, Alex"
greet("Alex", "Hello")     >> "Hello, Alex"
greet("Alex", NULL)        >> "Hi, Alex"   (NULL means "use the default")
greet()                    >> error: name is required
```

Optional inputs go after the required ones. Passing too many inputs is an error. Passing `NULL` skips an input in the middle, for built-in commands too: `RANDOM(1, 10, NULL, 42)`.

### Any number of inputs

Put `...` before the last input to collect every extra value into a table:

```text
DEF total(...nums) {
    LOCAL sum = 0
    FOR n IN nums { sum += n }
    RETURN sum
}
total(1, 2, 3)      >> 6
total()             >> 0
```

### Shared inputs

Inputs are copies, so a function can't accidentally change what you pass in. Mark an input with `@` to share it instead:

```text
DEF add_bonus(@p) { p.score += 10 }    >> changes the real table
DEF no_change(p) { p.score += 10 }     >> changes only its own copy

player = [score: 0]
add_bonus(player)      >> player.score is now 10
no_change(player)      >> player.score is still 10
```

`@` combines with defaults: `DEF heal(@p, amount = 10)`. See [copying and sharing](07-tables-and-text.md#copying-and-sharing).

## Functions as values

A function's name without `()` is the function itself. With `()`, it runs:

```text
DEF say_hi() { PRINT("hi") }

greet = say_hi         >> stores the function
greet()                >> prints "hi"

DEF do_twice(action) {
    action()
    action()
}
do_twice(say_hi)       >> passes a function into another function

double = DEF(x) { RETURN x * 2 }    >> a function with no name
PRINT(double(5))                     >> 10
```

- **Functions can be stored in tables:** `player = [greet: say_hi]`, then `player.greet()`.
- **Functions and variables share the same names.** If `score = 5` exists and you later write `DEF score()`, the function replaces the variable.
- **`PRINT` shows a function as `FUNCTION say_hi`,** or just `FUNCTION` for one with no name.
