// Tasks: functions running side by side.
//
//     t = START walk(hero)      starts walk(hero) as a task and goes straight on
//     AWAIT(t)                  waits for it to finish and gives back what it returned
//     DONE(t)                   TRUE once it has finished
//     CANCEL(t)                 stops it
//
// Tasks take turns: one runs at a time, and it hands over to the next after SLICE
// statements, or sooner when it WAITs or AWAITs. Turns go round in order, so the same
// program does the same thing every time (unless it WAITs for real time to pass), and
// two tasks can never change a variable at the same moment.
//
// An error in a task goes to whatever is AWAITing that task, where TRY can catch it.
// When nothing is, it stops the whole program, like an error anywhere else (so does
// END): the program's own code gets the next turn and stops there.
//
// Each task runs on its own thread, so it can pause anywhere, even deep inside its own
// function calls, but only the thread holding the baton touches the interpreter. The
// interpreter's values use Rc, which isn't meant to cross threads; that's sound here
// because the baton (a mutex and condition variable) means only one thread ever uses
// them at a time, with each hand-over ordering everything before it.

use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::interp::{Frame, Interp};
use crate::value::*;

/// Statements a task runs before the next one gets a turn.
pub const SLICE: u32 = 1000;

/// What a TASK value refers to.
pub struct TaskHandle {
    pub id: usize,
    pub name: Rc<str>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum State {
    Ready,
    Sleeping(Instant),
    Awaiting(usize),
    Finished,
}

pub struct TaskRec {
    pub name: Rc<str>,
    pub state: State,
    pub cancelled: bool,
    pub result: Value,
    /// While the task isn't running: its function calls.
    pub frames: Vec<Frame>,
    pub try_depth: usize,
    /// An error from a task this one is AWAITing.
    pub error: Option<Fault>,
}

impl TaskRec {
    fn new(name: Rc<str>) -> Self {
        TaskRec { name, state: State::Ready, cancelled: false, result: Value::Null, frames: Vec::new(), try_depth: 0, error: None }
    }
}

/// Whose turn it is.
pub struct Baton {
    turn: Mutex<usize>,
    cv: Condvar,
}

impl Baton {
    fn give(&self, to: usize) {
        *self.turn.lock().unwrap() = to;
        self.cv.notify_all();
    }
    fn wait_for(&self, me: usize) {
        let mut t = self.turn.lock().unwrap();
        while *t != me {
            t = self.cv.wait(t).unwrap();
        }
    }
}

/// Moves something to a task's thread. Only sound because of the baton (see the top).
struct SendIt<T>(T);
unsafe impl<T> Send for SendIt<T> {}

impl Interp {
    /// START: runs `func` with its inputs (already worked out) as a new task.
    pub fn start_task(&mut self, func: Rc<Function>, vals: Vec<Slot>, line: usize) -> R<Value> {
        if self.tasks.is_empty() {
            self.tasks.push(TaskRec::new(Rc::from("the program")));
            self.baton = Some(Arc::new(Baton { turn: Mutex::new(0), cv: Condvar::new() }));
        }
        let id = self.tasks.len();
        let name: Rc<str> = Rc::from(func.decl.name.as_deref().unwrap_or("FUNCTION"));
        let mut rec = TaskRec::new(name.clone());
        // The task's first call, ready to go.
        rec.frames.push(Frame { func: Some(func.clone()), scopes: vec![OrdMap::new()], env: func.env.clone(), line, src: func.src.clone() });
        self.tasks.push(rec);
        let baton = self.baton.clone().unwrap();
        let job = SendIt((self as *mut Interp, func, vals));
        let spawned = std::thread::Builder::new().stack_size(256 * 1024 * 1024).spawn(move || {
            let job = job;
            let (ptr, func, vals) = job.0;
            baton.wait_for(id);
            // SAFETY: this thread holds the baton, so nothing else is using the interpreter,
            // which lives until the program ends (and the program ends by exiting).
            let it = unsafe { &mut *ptr };
            let r = if it.tasks[id].cancelled { Ok(Value::Null) } else { it.run_task(&func, vals) };
            it.task_finished(id, r);
        });
        if spawned.is_err() {
            self.tasks.pop();
            return err("couldn't start another task (the computer is out of threads)");
        }
        Ok(Value::Task(Rc::new(TaskHandle { id, name })))
    }

    fn run_task(&mut self, func: &Rc<Function>, vals: Vec<Slot>) -> R<Value> {
        let decl = func.decl.clone();
        let r = self.run_call(func, &decl, vals);
        self.frames.clear();
        r
    }

    /// A task's function has ended: keep its result (an error or END goes to the
    /// program's own code, see the top), then hand over for good.
    fn task_finished(&mut self, id: usize, r: R<Value>) {
        match r {
            Ok(v) => self.tasks[id].result = v,
            Err(Fault::Cancelled) => {}
            Err(Fault::Error(e)) => match self.tasks.iter().position(|t| t.state == State::Awaiting(id)) {
                Some(w) => self.tasks[w].error = Some(Fault::Error(e)),
                None => {
                    self.task_fault.get_or_insert(Fault::TaskError(e));
                }
            },
            Err(f) => {
                self.task_fault.get_or_insert(f);
            }
        }
        self.tasks[id].state = State::Finished;
        self.frames.clear();
        let next = match self.pick_next(id) {
            Ok(n) => n,
            Err(f) => {
                // Every other task is stuck: the program's code gets the error.
                self.task_fault.get_or_insert(f);
                0
            }
        };
        self.hand_over(id, next);
    }

    /// Lets the other tasks have a turn; comes back when it's this task's turn again.
    pub fn take_turns(&mut self) -> R<()> {
        let me = self.current;
        let next = self.pick_next(me)?;
        if next != me {
            self.hand_over(me, next);
            self.baton.clone().unwrap().wait_for(me);
            // Whoever handed back loaded this task's calls (see hand_over).
        }
        self.steps = 0;
        if self.tasks[me].cancelled {
            return Err(Fault::Cancelled);
        }
        if me == 0 {
            if let Some(f) = self.task_fault.take() {
                return Err(f);
            }
        }
        Ok(())
    }

    /// Saves the running task's calls, loads the next one's, and gives it the baton.
    fn hand_over(&mut self, from: usize, to: usize) {
        self.tasks[from].frames = std::mem::take(&mut self.frames);
        self.tasks[from].try_depth = self.try_depth;
        self.frames = std::mem::take(&mut self.tasks[to].frames);
        self.try_depth = self.tasks[to].try_depth;
        self.current = to;
        self.steps = 0;
        self.baton.as_ref().unwrap().give(to);
    }

    /// The next task to run after `me`, in turn order (`me` itself last). When every
    /// task is WAITing, sleeps until the first one wakes.
    fn pick_next(&mut self, me: usize) -> R<usize> {
        loop {
            // A task's error goes straight to the program's own code.
            if self.task_fault.is_some() && self.tasks[0].state != State::Finished {
                self.tasks[0].state = State::Ready;
                return Ok(0);
            }
            let now = Instant::now();
            for i in 0..self.tasks.len() {
                match self.tasks[i].state {
                    State::Sleeping(t) if t <= now => self.tasks[i].state = State::Ready,
                    State::Awaiting(x) if self.tasks[x].state == State::Finished => self.tasks[i].state = State::Ready,
                    _ => {}
                }
                // A cancelled task gets a turn to end.
                if self.tasks[i].cancelled && self.tasks[i].state != State::Finished {
                    self.tasks[i].state = State::Ready;
                }
            }
            let n = self.tasks.len();
            for k in 1..=n {
                let c = (me + k) % n;
                if self.tasks[c].state == State::Ready {
                    return Ok(c);
                }
            }
            let wake = self.tasks.iter().filter_map(|t| if let State::Sleeping(w) = t.state { Some(w) } else { None }).min();
            match wake {
                Some(w) => {
                    use std::io::Write;
                    let _ = self.out.flush();
                    std::thread::sleep(w.saturating_duration_since(now));
                }
                None => return err("every task is waiting for another one to finish (AWAIT), so none of them can go on"),
            }
        }
    }

    /// Counts a statement; every SLICE statements, the next task gets a turn.
    pub fn tick(&mut self) -> R<()> {
        self.steps += 1;
        if self.steps >= SLICE {
            self.take_turns()?;
        }
        Ok(())
    }

    /// WAIT(seconds) while there are tasks: the others run meanwhile.
    pub fn task_sleep(&mut self, secs: f64) -> R<()> {
        let me = self.current;
        self.tasks[me].state = State::Sleeping(Instant::now() + Duration::from_secs_f64(secs.clamp(0.0, 1e9)));
        let r = self.take_turns();
        self.tasks[me].state = State::Ready;
        r
    }

    fn task_id(&self, v: &Value, what: &str) -> R<usize> {
        match v {
            Value::Task(h) => Ok(h.id),
            o => err(format!("{} needs a TASK (from START), not a {}", what, o.type_name())),
        }
    }

    /// AWAIT(task) or AWAIT(table of tasks): waits for them, and gives back what they returned.
    pub fn await_tasks(&mut self, v: &Value) -> R<Value> {
        if let Value::Table(t, _) = v {
            let mut out = Vec::new();
            for s in &t.items {
                out.push(self.await_tasks(&s.borrow().clone())?);
            }
            return Ok(Value::new_table(Table::from_values(out)));
        }
        let id = self.task_id(v, "AWAIT")?;
        let me = self.current;
        if id == me {
            return err("a task can't AWAIT itself (it would wait forever)");
        }
        while self.tasks[id].state != State::Finished {
            self.tasks[me].state = State::Awaiting(id);
            let r = self.take_turns();
            self.tasks[me].state = State::Ready;
            r?;
        }
        if let Some(f) = self.tasks[me].error.take() {
            return Err(f);
        }
        Ok(self.tasks[id].result.clone())
    }

    pub fn task_done(&self, v: &Value) -> R<bool> {
        let id = self.task_id(v, "DONE")?;
        Ok(self.tasks[id].state == State::Finished)
    }

    pub fn cancel_task(&mut self, v: &Value) -> R<()> {
        let id = self.task_id(v, "CANCEL")?;
        if id == self.current {
            return err("a task can't CANCEL itself; use RETURN to end it");
        }
        if self.tasks[id].state != State::Finished {
            self.tasks[id].cancelled = true;
            // Let it stop now, so it's DONE when CANCEL comes back.
            self.await_tasks(v)?;
        }
        Ok(())
    }

    /// After an error in Quick-try: stops every task that's still going.
    pub fn cancel_all_tasks(&mut self) {
        self.task_fault = None;
        for t in self.tasks.iter_mut().skip(1) {
            if t.state != State::Finished {
                t.cancelled = true;
            }
        }
        let _ = self.finish_tasks();
        self.task_fault = None;
        if !self.tasks.is_empty() {
            self.tasks[0].state = State::Ready;
        }
    }

    /// The program's own code has ended: let the tasks it started finish.
    pub fn finish_tasks(&mut self) -> R<()> {
        while let Some(id) = (1..self.tasks.len()).find(|&i| self.tasks[i].state != State::Finished) {
            self.await_tasks(&Value::Task(Rc::new(TaskHandle { id, name: self.tasks[id].name.clone() })))?;
        }
        Ok(())
    }
}
