# 11. Tasks: things happening at the same time

A task is a function running side by side with the rest of the program. `START` begins one and goes straight on:

```text
DEF countdown(name, n) {
    FOR i IN n TO 1 STEP -1 {
        PRINT(name, i)
        WAIT(0.5)
    }
    RETURN name + " done"
}

a = START countdown("rocket", 3)
b = START countdown("timer", 2)
PRINT("both started")
PRINT(AWAIT(a), AWAIT(b))
```

| Command | What it does |
| --- | --- |
| `START f(x, y)` | runs `f(x, y)` as a task, and gives back the task (a `TASK` value) right away |
| `AWAIT(task)` | waits until the task has finished, and gives back what it returned (`NULL` if nothing) |
| `AWAIT([t1, t2])` | waits for every task in a table, and gives back a table of what they returned |
| `DONE(task)` | `TRUE` once the task has finished |
| `CANCEL(task)` | stops the task (it's finished, with `NULL` as its result, when `CANCEL` comes back) |
| `WAIT(seconds)` | while tasks are running, the others carry on during the wait |

## How tasks take turns

- **One runs at a time, and they take turns.** A task runs about a thousand statements, then the next one gets a turn. `WAIT` and `AWAIT` hand over straight away.
- **Turns go round in order,** so the same program does the same thing every time (unless it `WAIT`s for real time to pass). Both interpreters take turns the same way.
- **Two tasks never change a variable at the same moment,** so `box.count += 1` from several tasks always adds up. A task can still see a value change between its turns.
- **Tasks share the program's outside variables,** like any function (reading them, changing table entries, or `GLOBAL` to give them a value). Inputs work as usual: copies, or shared with `@` in the function's inputs.
- **When the program's own code ends, the tasks it started get to finish** before it stops. `END` stops everything at once.
- **Tasks can start tasks,** and any task can `AWAIT` any other one.

## Errors in tasks

- **An error in a task goes to whatever is `AWAIT`ing that task,** so `TRY` around the `AWAIT` catches it:

  ```text
  DEF load_level() { RETURN READ("level9.txt") }
  t = START load_level()
  TRY {
      level = AWAIT(t)
  } FAIL err {
      PRINT("couldn't load:", err.message)
  }
  ```

- **When nothing is `AWAIT`ing the task, the error stops the program,** like an error anywhere else. (A `TRY` the program happens to be in doesn't catch it, since it's about something else.)
- **A task that only `AWAIT`s tasks that are waiting for it can never go on.** When every task is stuck like that, it's an error: "every task is waiting for another one to finish (AWAIT), so none of them can go on".
- **A task can't `AWAIT` or `CANCEL` itself.** Use `RETURN` to end the task you're in.

`TYPE(task)` is `"TASK"`, and `PRINT(task)` shows `TASK` and the function's name.
