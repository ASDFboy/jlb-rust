# 6. Functions

## Defining and calling

```text
FUNC add(a, b) {
    RETURN a + b
}
PRINT(add(2, 3))      >> 5
```

- **A function without `RETURN` gives `NULL`.**
- **`RETURN` can only be used inside a function.**
- **Functions are defined in order as the file runs,** so calling one before its `FUNC` line has run is an error. A function can still call another function defined further down, as long as that one exists by the time the call actually happens.
- **A function can call itself.** About 2,500 calls deep, it stops with an error instead of crashing.

## Scope: which variables a function sees

A function sees its own local variables and the file's outside variables:

- **Assigning to a name that already exists outside the function changes the outside variable.**
- **A name first created inside the function is local,** and disappears when the function finishes.

```text
score = 10
FUNC add_points(amount) {
    score += amount          >> changes the outside score
    bonus = amount * 2       >> new name, so it's local
    RETURN bonus
}
```

### LOCAL

`LOCAL` makes a local variable even when an outside variable has the same name:

```text
count = 3
FUNC count_enemies() {
    LOCAL count = 0          >> separate from the outside count
    FOR e IN enemies { count += 1 }
    RETURN count
}
>> outside, count is still 3
```

`LOCAL x` on its own starts as `NULL`. `LOCAL` can only be used inside a function.

### What counts as local

A function's locals are:

- anything declared with `LOCAL`;
- any new name first created inside it;
- its inputs.

A `FOR` loop's variables (and a `FAIL` block's error name) are different: they exist only inside that loop's `{ }` block, in or out of a function.

### Reaching another function's locals

Inside its own function, a local is used by its short name. Everywhere else, put the function's name in front:

```text
FUNC outer() {
    LOCAL secret = 42
    RETURN inner()
}
FUNC inner() {
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
FUNC greet(name, greeting = "Hi") {
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
FUNC total(...nums) {
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
FUNC add_bonus(@p) { p.score += 10 }    >> changes the real table
FUNC no_change(p) { p.score += 10 }     >> changes only its own copy

player = [score: 0]
add_bonus(player)      >> player.score is now 10
no_change(player)      >> player.score is still 10
```

`@` combines with defaults: `FUNC heal(@p, amount = 10)`. See [copying and sharing](07-tables-and-text.md#copying-and-sharing).

## Functions as values

A function's name without `()` is the function itself. With `()`, it runs:

```text
FUNC say_hi() { PRINT("hi") }

greet = say_hi         >> stores the function
greet()                >> prints "hi"

FUNC do_twice(action) {
    action()
    action()
}
do_twice(say_hi)       >> passes a function into another function

double = FUNC(x) { RETURN x * 2 }    >> a function with no name
PRINT(double(5))                     >> 10
```

- **Functions can be stored in tables:** `player = [greet: say_hi]`, then `player.greet()`.
- **Functions and variables share the same names.** If `score = 5` exists and you later write `FUNC score()`, the function replaces the variable.
- **`PRINT` shows a function as `FUNC say_hi`,** or just `FUNC` for one with no name.
