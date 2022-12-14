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
}

impl Server {
    /// Construct a new server, initially in the idle state
    pub fn new() -> Server {
        Server {
            state: State::Idle,
            flash_error: 0,
            flash_warn: 0,
        }
    }

    /// (Attempt to) start a new block 
    pub fn start_block(&mut self, duration_s: u64) {
        match self.state {
            State::Idle => {
                self.state = State::InBlock {
                    end_time: std::time::Instant::now() + std::time::Duration::from_secs(duration_s),
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
        match self.state {
            State::InBlock { .. } => self.state = State::Idle,
            _ => self.flash_warn = 5,
        }
    }

    /// Attempt to pause a currently-running block
    pub fn pause_block(&mut self) {
        match self.state {
            State::InBlock { end_time } => {
                self.state = State::Paused {
                    duration: end_time - std::time::Instant::now(),
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
            State::Idle => format!("<fc=#888{}>--</fc>", bg_col),
            State::Paused { duration } => {
                let rem = duration.as_secs();
                format!("<fc=#0F0{}>{:02}:{:02}</fc>", bg_col, rem / 60, rem % 60)
            }
            State::InBlock { end_time } => {
                if now > end_time {
                    self.state = State::InCooldown { end_time: now + crate::COOLDOWN_DURATION };
                };
                let rem = (end_time - now).as_secs();
                if rem < 10 && rem % 2 == 1 {
                    bg_col = ",#FF0";
                }
                format!("<fc=#0F0{}>{:02}:{:02}</fc>", bg_col, rem / 60, rem % 60)
            },
            State::InCooldown { end_time } => {
                if now > end_time {
                    self.state = State::Idle;
                };

                let rem = (end_time - now).as_secs();
                if rem < 10 && rem % 2 == 1 {
                    bg_col = ",#FF0";
                }
                format!("<fc=#FF0{}>{:02}:{:02}</fc>", bg_col, rem / 60, rem % 60)
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
        end_time: std::time::Instant,
    },
    /// Timer is paused
    Paused {
        duration: std::time::Duration,
    },
    /// The server is counting down the post-block cooldown
    InCooldown {
        end_time: std::time::Instant,
    },
}

