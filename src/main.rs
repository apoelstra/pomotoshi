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

//! Pomotoshi
//!
//! Strongly inspired by [Pomobar](https://github.com/rlcintra/pomobar), this
//! utility is a Pomodoro timer, designed to be controlled via DBus and to
//! provide output via xmobar.
//!

mod color;
mod server;
mod task;

use dbus::blocking::LocalConnection;
use dbus::channel::MatchingReceiver;
use dbus_crossroads::{Crossroads, Context};
use std::process::Command;
use std::sync::{Arc, Mutex};

/// How long cooldown (period after a block when no new blocks are allowed) should last
const COOLDOWN_DURATION: std::time::Duration = std::time::Duration::from_secs(300);

/// Frequency with which to update xmobar
///
/// This should be less than a second to ensure that the clock/timer is updated
/// every second, but is otherwise more-or-less arbitrary. It does define the
/// flashing speed so it probably should not be super low.
const UPDATE_FREQ: std::time::Duration = std::time::Duration::from_millis(100);
/// Name of the D-Bus org
const DBUS_ORG: &str = "org.Pomotoshi";
/// Name of the D-Bus path
const DBUS_PATH: &str = "/org/pomotoshi";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Arc::new(Mutex::new(server::Server::new()));

    // Start D-Bus connection
    let c = LocalConnection::new_session()?;
    // See https://dbus.freedesktop.org/doc/api/html/group__DBusBus.html for documentation
    // of flags, which for some reason are exposed by the Rust API as undocumented booleans.
    c.request_name(
        // Name
        DBUS_ORG,
        // DBUS_NAME_FLAG_ALLOW_REPLACEMENT -- don't allow other instances to replace us
        false,
        // DBUS_NAME_FLAG_REPLACE_EXISTING -- don't try to replace other instances
        false,
        // DBUS_NAME_FLAG_DO_NOT_QUEUE -- if another instance exists, just fail
        true,
    )?;

    // Setup Crossroads instance
    let mut cr = Crossroads::new();

    let iface_token = cr.register(DBUS_ORG, |b| {
        // startBlock method: takes an integer number of time, in seconds
        b.method(
            "startBlock", // name
            ("time_s",), // input args
            (), // output args
            move |_: &mut Context, server: &mut Arc<Mutex<server::Server>>, (time_s,): (u64,)| {
                let mut lock = server.lock()
                    .expect("server did not witness a panic");
                lock.start_block(time_s);
                Ok(())
            },
        );
        b.method(
            "cancelBlock", // name
            (), // input args
            (), // output args
            move |_: &mut Context, server: &mut Arc<Mutex<server::Server>>, _: ()| {
                let mut lock = server.lock()
                    .expect("server did not witness a panic");
                lock.cancel_block();
                Ok(())
            },
        );
        b.method(
            "pauseBlock", // name
            (), // input args
            (), // output args
            move |_: &mut Context, server: &mut Arc<Mutex<server::Server>>, _: ()| {
                let mut lock = server.lock()
                    .expect("server did not witness a panic");
                lock.pause_block();
                Ok(())
            },
        );
        b.method(
            "blockLog", // name
            (), // input args
            ("log",), // output args
            move |_: &mut Context, server: &mut Arc<Mutex<server::Server>>, _: ()| {
                let mut lock = server.lock()
                    .expect("server did not witness a panic");
                Ok((lock.block_log(),))
            },
        );
        b.method(
            "taskLogAdd", // name
            ("name",), // input args
            (), // output args
            move |_: &mut Context, server: &mut Arc<Mutex<server::Server>>, (name,): (String,)| {
                let mut lock = server.lock()
                    .expect("server did not witness a panic");
                lock.task_log_add(name);
                Ok(())
            },
        );
        b.method(
            "taskLogRemove", // name
            ("name",), // input args
            (), // output args
            move |_: &mut Context, server: &mut Arc<Mutex<server::Server>>, (name,): (String,)| {
                let mut lock = server.lock()
                    .expect("server did not witness a panic");
                lock.task_log_remove(&name);
                Ok(())
            },
        );
        b.method(
            "taskLogOutput", // name
            ("name",), // input args
            ("log",), // output args
            move |_: &mut Context, server: &mut Arc<Mutex<server::Server>>, (name,): (String,)| {
                let mut lock = server.lock()
                    .expect("server did not witness a panic");
                Ok((lock.task_log_dump(&name),))
            },
        );
    });
    cr.insert(DBUS_PATH, &[iface_token], Arc::clone(&server));

    // Serve clients forever.
    // We add the Crossroads instance to the connection so that incoming method calls will be handled.
    c.start_receive(dbus::message::MatchRule::new_method_call(), Box::new(move |msg, conn| {
        cr.handle_message(msg, conn).unwrap();
        true
    }));

    // Serve clients forever.
    loop {
        // D-Bus updates
        c.process(UPDATE_FREQ)?;

        let mut lock = server.lock()
            .expect("server did not witness a panic");

        // Record currently-active window
        let curr_win = Command::new("xdotool")
            .arg("getwindowfocus")
            .arg("getwindowname")
            .output()
            .expect("executing xdotool")
            .stdout;
        lock.record_current_window(String::from_utf8_lossy(&curr_win).as_ref());

        // Output state to xmobar
        println!("{}", lock.xmobar_update());
    }
}

