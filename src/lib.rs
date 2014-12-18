extern crate collections;

use collections::RingBuf;

impl<T: GetTimestamp> TimeContext<T> {

    /// Creates new `TimeContext` which will use a user-supplied `GetTimestamp` struct.
    #[stable]
    pub fn new(timefunc: T) -> TimeContext<T> {
        TimeContext {
            timefunc: timefunc,
            curspeed: 0f64,
            cprog: 0f64,
            fprog: None,
            fts: None,
            samples: RingBuf::with_capacity(10),
        }
    }

    /// Updates the state of this `TimeContext` instance.
    ///
    /// `cur_prog` argument is current progress value for an operation. `max_prog` is a value that
    /// symbolizes a maximum progress value for an operation. When `cur_prog` reaches `max_prog`,
    /// `gaeta` considers the progress to be 100%.
    ///
    /// This method should be called periodically by your busy loop function in order to
    /// property calculate ETA. Internally it will call `GetTimestamp`'s `get_timestamp` method
    /// in order to read current time.
    #[unstable]
    pub fn update_eta(&mut self, cur_prog: u64, max_prog: u64) {
        if let None = self.fts {
            self.fts = Some(self.timefunc.get_timestamp());
        }

        self.update_history(cur_prog, max_prog);
        self.curspeed = self.calc_speed_per_unit();
    }

    fn update_history(&mut self, cur_prog: u64, max_prog: u64) {
        let prog = self.get_progress(cur_prog, max_prog);
        self.cprog = prog;

        if let None = self.fprog {
            self.fprog = Some(prog);
        }

        let vhist = &mut self.samples;
        let ts = self.timefunc.get_timestamp();

        let last_progress = if let Some(tv) = vhist.get(vhist.len() - 1) {
            tv.progress
        } else {
            0f64
        };

        if last_progress == prog - self.fprog.unwrap() {
            return;
        }

        if vhist.len() > 9 {
            vhist.pop_front();
        }

        vhist.push_back(Sample {
            timestamp: ts,
            progress: prog
        });
    }

    /// Returns the currently measured speed.
    ///
    /// The return value can be interpreted as: `x%` for every unit of time. In other words, if
    /// `GetTimestamp` returns time units measured in seconds, the result of `calc_speed_per_unit`
    /// can be interpreted as: `x%` for every second.
    ///
    /// The unit of time is the same as chosen by the `GetTimestamp`'s `get_timestamp` method.
    #[unstable]
    pub fn calc_speed_per_unit(&self) -> f64 {
        if self.samples.len() == 0 {
            return 0f64;
        }

        let mut speed_sum = 0f64;
        for vitem in self.samples.iter() {
            let timestamp = vitem.timestamp - self.fts.unwrap_or(0u64);
            let percent = vitem.progress - self.fprog.unwrap_or(0f64);

            let speed_sample = percent as f64 / if timestamp == 0 { 1 } else { timestamp } as f64;
            speed_sum = speed_sum + speed_sample;
        }

        let speed = speed_sum / self.samples.len() as f64;
        speed
    }

    fn get_progress(&self, cur: u64, max: u64) -> f64 {
        ((cur as f64) * 100.0f64 / (max as f64))
    }

    /// Returns the remaining time (ETA).
    ///
    /// The unit of time is the same as chosen by the `GetTimestamp`'s `get_timestamp` method.
    #[unstable]
    pub fn get_remaining_time(&self) -> int {
        if let None = self.fts { return 0; }
        if let None = self.fprog { return 0; }

        let remaining_prc = 100.0f64 - (self.cprog - self.fprog.unwrap_or(0f64));
        let whole_work = remaining_prc / self.curspeed;

        if whole_work >= 0f64 {
            whole_work as int
        } else {
            0i
        }
    }

    /// Gets a reference to the underlying `GetTimestamp` struct, which was set by the `new`
    /// constructor.
    #[experimental]
    pub fn get_timefunc(&self) -> &T { &self.timefunc }

    /// Gets a mutable reference to the underlying `GetTimestamp` struct, which was set by the
    /// `new` constructor.
    #[experimental]
    pub fn get_timefunc_mut(&mut self) -> &mut T { &mut self.timefunc }
}

/// A trait that describes a callback mechanism for `gaeta` to read the current time.
///
/// # Example of a valid implementation of `GetTimestamp` trait
///
/// ```rust
///   extern crate time;
///
///   struct SystemTimer;
///
///   impl SystemTimer {
///       fn new() -> SystemTimer { SystemTimer }
///   }
///
///   // Trait implementation.
///   impl GetTimestamp for SystemTimer {
///       fn get_timestamp(&self) -> u64 {
///           // Converts nanosecond to a millisecond.
///
///           time::precise_time_ns() / 1_000_000
///       }
///   }
///
/// ```
///
/// This example implementation chooses a millisecond to be the unit of time used by the library.
#[experimental]
pub trait GetTimestamp {
    fn get_timestamp(&self) -> u64;
}

struct Sample {
    timestamp: u64,
    progress: f64,
}

/// Wraps ETA calculation methods into one struct.
///
/// It is parametrized by the `GetTimestamp` trait. Users should create their own structure which
/// implements this trait.
///
/// See the description of `GetTimestamp` trait for an example of a valid implementation of this
/// trait.
///
/// This way the `gaeta` library doesn't create any dependencies by itself. The user also can
/// choose the best way to get the timestamp.
#[experimental]
pub struct TimeContext<T> {
    timefunc: T,          // An interface that provides current time in some unit.
    curspeed: f64,        // Current speed per time unit.
    fts: Option<u64>,     // First timestamp.
    fprog: Option<f64>,   // First progress.
    cprog: f64,           // Current progress.
    samples: RingBuf<Sample>,
                          // A small table of recent time samples, to make the result more smooth.
}


/// A trait that describes a testing callback mechanism for `gaeta` to read a value that symbolizes
/// passing time.
///
/// It is used in unit testing.
#[stable]
#[deriving(Copy)]
pub struct TestTimer {
    cur_ts: u64,
}

// For testing purposes.

#[stable]
impl TestTimer {
    /// Creates a new instance of this struct.
    #[stable]
    pub fn new() -> TestTimer {
        TestTimer {
            cur_ts: 0u64,
        }
    }

    /// Sets the value to be returned by `get_timestamp` method.
    #[stable]
    pub fn set_timestamp(&mut self, ts: u64) {
        self.cur_ts = ts;
    }
}

#[stable]
impl GetTimestamp for TestTimer {
    /// Returns the value that was set by `set_timestamp` method.
    fn get_timestamp(&self) -> u64 {
        self.cur_ts
    }
}
