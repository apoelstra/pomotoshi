// Pomotoshi
// Written in 2022 by
//   Andrew Poelstra <icboc@wpsoftware.net>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! Server
//!
//! The data managed by the actual timer
//!

/// Main server structure
pub struct Server {
    /// The current state
    state: State,
    /// Error flash countdown
    flash_error: usize,
    /// Warning flash countdown
    flash_warn: usize,
    /// Last active-window-log update
    last_task_report: std::time::Instant,
    /// Log of active windows
    task_log: crate::task::Task,
    /// Log of block start/stop/etc
    block_log: String,
    /// Log of active windows (which must be manually reset)
    long_task_log: crate::task::Task,
}

impl Server {
    /// Construct a new server, initially in the idle state
    pub fn new() -> Server {
        Server {
            state: State::Idle,
            flash_error: 0,
            flash_warn: 0,
            last_task_report: std::time::Instant::now(),
            task_log: crate::task::Task::new_root(),
            long_task_log: crate::task::Task::new_root(),
            block_log: String::new(),
        }
    }

    /// Internal logging method
    fn log(&mut self, log_str: &str) {
        let date = std::process::Command::new("date")
            .arg("+%F %T%z")
            .output()
            .expect("executing bash")
            .stdout;
        let date = String::from_utf8_lossy(&date);
        self.block_log += &format!("{}: {}\n", date, log_str);
    }

    /// Record the current active window, for task-tracking purposes
    pub fn record_current_window(&mut self, win: &str) {
        if let State::InBlock { .. } = self.state {
            let duration = std::time::Instant::now() - self.last_task_report;
            self.last_task_report = std::time::Instant::now();
            self.task_log.add_time(win, duration);
            self.long_task_log.add_time(win, duration);
        }
    }

    /// Output statistics from "short" log
    pub fn dump_stats(&mut self, reset: bool) -> String {
        if reset {
            self.log("reset statistics");
            self.task_log = crate::task::Task::new_root();
        } else {
            self.log("output statistics (did not reset)");
        }
        format!("{}{}", self.block_log, self.task_log.to_string())
    }

    /// Output statistics from "long" log
    pub fn dump_long_stats(&mut self, reset: bool) -> String {
        if reset {
            self.log("reset long statistics");
            self.task_log = crate::task::Task::new_root();
        } else {
            self.log("output long statistics (did not reset)");
        }
        format!("{}{}", self.block_log, self.task_log.to_string())
    }

    /// (Attempt to) start a new block 
    pub fn start_block(&mut self, duration_s: u64) {
        self.block_log = String::new();
        self.log("started block");
        match self.state {
            State::Idle => {
                let duration = std::time::Duration::from_secs(duration_s);
                self.task_log = crate::task::Task::new_root();
                self.state = State::InBlock {
                    duration,
                    end_time: std::time::Instant::now() + duration,
                };
            },
            State::Paused { .. } | State::InBlock { .. } => {
                // refuse te start a block when one is running; first cancel the running one
                self.flash_warn = 5;
            },
            State::InCooldown { .. } => {
                // refuse te start a block during cooldown; cooldown cannot be cancelled.
                self.flash_error = 7;
            },
        }
    }

    /// Attempt to cancel a currently-running block
    pub fn cancel_block(&mut self) {
        self.log("canceled block");
        match self.state {
            State::InBlock { .. } => self.state = State::Idle,
            State::InCooldown { .. } => self.flash_error = 7,
            _ => self.flash_warn = 5,
        }
    }

    /// Attempt to pause a currently-running block
    pub fn pause_block(&mut self) {
        match self.state {
            State::InBlock { duration, end_time } => {
                self.log("paused block");
                self.state = State::Paused {
                    total_duration: duration,
                    remaining_duration: end_time - std::time::Instant::now(),
                };
            },
            State::Paused { total_duration, remaining_duration } => {
                self.log("unpaused block");
                self.state = State::InBlock {
                    duration: total_duration,
                    end_time: std::time::Instant::now() + remaining_duration,
                };
            },
            _ => self.flash_warn = 5,
        }
    }

    /// Write a single line of output to xmobar
    pub fn xmobar_update(&mut self) -> String {
        let now = std::time::Instant::now();
        let mut bg_col = "";
        // Flash a warning, if one is happening
        if self.flash_warn > 0 {
            if self.flash_warn % 2 == 1 {
                bg_col = ",#FF0";
            }
            self.flash_warn -= 1;
        }
        // Flash an error, if one is happening
        if self.flash_error > 0 {
            if self.flash_error % 2 == 1 {
                bg_col = ",#F00";
            }
            self.flash_error -= 1;
        }
        // Actually display status
        match self.state {
            State::Idle => format!("<fc=#AAA{}>--</fc>", bg_col),
            State::Paused { remaining_duration, .. } => {
                let rem = remaining_duration.as_secs();
                format!("<fc=#AAA{}>{:02}:{:02}</fc>", bg_col, rem / 60, rem % 60)
            }
            State::InBlock { end_time, duration } => {
                if now > end_time {
                    self.log("end block; start cooldown");
                    self.state = State::InCooldown { end_time: now + crate::COOLDOWN_DURATION };
                };
                let rem_duration = end_time - now;
                let rem_s = rem_duration.as_secs();
                if rem_s < 10 && rem_s % 2 == 1 {
                    bg_col = ",#FF0";
                }
                format!(
                    "<fc={}{}>{:02}:{:02}</fc>",
                    crate::color::fade_between((255, 255, 0), (0, 255, 0), rem_duration, duration),
                    bg_col,
                    rem_s / 60,
                    rem_s % 60,
                )
            },
            State::InCooldown { end_time } => {
                if now > end_time {
                    self.log("end cooldown");
                    // FIXME we probably shouldn't hardcode this
                    std::process::Command::new("bash")
                        .arg("-c")
                        .arg("source ~/.bashrc && keyboard.sh")
                        .output()
                        .expect("executing bash");
                    self.state = State::Idle;
                };

                let rem_duration = end_time - now;
                let rem_s = rem_duration.as_secs();
                if rem_s < 10 && rem_s % 2 == 1 {
                    bg_col = ",#FF0";
                }
                format!(
                    "<fc={}{}>{:02}:{:02}</fc>",
                    crate::color::fade_between((0, 255, 255), (255, 0, 0), rem_duration, crate::COOLDOWN_DURATION),
                    bg_col,
                    rem_s / 60,
                    rem_s % 60,
                )
            },
        }
    }
}

/// The state machine
#[derive(PartialEq, Eq)]
enum State {
    /// The server is idle (no current block)
    Idle,
    /// The server is counting down a given block
    InBlock {
        duration: std::time::Duration,
        end_time: std::time::Instant,
    },
    /// Timer is paused
    Paused {
        total_duration: std::time::Duration,
        remaining_duration: std::time::Duration,
    },
    /// The server is counting down the post-block cooldown
    InCooldown {
        end_time: std::time::Instant,
    },
}

