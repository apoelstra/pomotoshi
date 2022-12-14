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

//! Tasks
//!
//! Structures describing tasks, and sub-tasks, which are inferred from
//! the title of the active window and used for time-tracking
//!

use regex::Regex;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::iter;
use std::time::Duration;

/// Node in the tree of "units of work"
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    focus_time: Duration,
    children: HashMap<String, Task>,
}

impl Task {
    /// Create a new primary task
    pub fn new_root() -> Task {
        Task {
            focus_time: Duration::default(),
            children: HashMap::new(),
        }
    }

    /// Add time to a task, specified by a path made of string indices
    ///
    /// The string indices are given in reverse order so that we can efficiently
    /// pop them off as a stack. In general, this function shouldn't be used
    /// directly; it is easier to use `add_time` which parses a window title
    /// into an appropriately formed path for you.
    pub fn add_time_path(&mut self, mut path: Vec<String>, time: Duration) {
        self.focus_time += time;
        if let Some(child) = path.pop() {
            self.children.entry(child)
                .or_insert(Task::new_root())
                .add_time_path(path, time);
        }
    }

    /// Add time to a task, specified by its window title
    pub fn add_time(&mut self, title: &str, time: Duration) {
        // FIXME actually split up the title
        self.add_time_path(title_to_path(title), time);
    }

    /// Stringify an individual task
    fn to_string_internal(&self, name: &str, indent: usize, total_s: f64) -> String {
        let focus_s = self.focus_time.as_millis() as f64 / 1000.0;
        let focus_pcnt = 100.0 * focus_s / total_s;

        let mut ret = String::new();
        ret.extend(iter::repeat(' ').take(indent));
        ret += &format!("- [{:5.2}% {:6.2}s] {}\n", focus_pcnt, focus_s, name.trim());
        let mut sorted_children: Vec<_> = self.children.iter().collect();
        sorted_children.sort_by_key(|(_, c)| -(c.focus_time.as_millis() as i64));
        for (name, child) in sorted_children {
            ret += &child.to_string_internal(name, indent + 4, total_s);
        }
        ret
    }

    /// Stringify (as a multi-line string) the task and all its children
    pub fn to_string(&self) -> String {
        let focus_s = self.focus_time.as_millis() as f64 / 1000.0;
        self.to_string_internal("", 0, focus_s)
    }
}

fn title_to_path(title: &str) -> Vec<String> {
    // Blockstream-specific qutebrowser
    if title.contains(" - qutebrowser") {
        if title.contains("Rocket.Chat") {
            return vec!["Rocket.Chat".into(), "Blockstream".into()];
        }
        if title.contains("Blockstream Mail") {
            return vec!["Gmail".into(), "Blockstream".into()];
        }
        if title.contains("Blockstream - Calendar") {
            return vec!["Calendar".into(), "Blockstream".into()];
        }
    }

    // Github-specific qutebrowser
    if title.contains("Notifications - qutebrowser") {
            return vec!["Notifications".into(), "Github".into()];
    }
    let github_regex = Regex::new(r"(?:\[\d{1,2}%\] )?(.*) · (Pull Request|Issue|Discussion) (#\d*) · (.*) - qutebrowser").unwrap();
    if let Some(github) = github_regex.captures(title) {
        return vec![format!("{} {}", &github[3], &github[1]), github[2].into(), github[4].into(), "Github".into()];
    }

    // General qutebrowser
    let qute_regex = Regex::new(r"(?:\[\d{1,2}%\] )?(.*) - (qutebrowser)").unwrap();
    if let Some(qute) = qute_regex.captures(title) {
        return vec![qute[1].into(), qute[2].into()];
    }

    // TMux
    let tmux_regex = Regex::new(r"(.*) \(tmux:(.*)/(.*)\)").unwrap();
    if let Some(tmux) = tmux_regex.captures(title) {
        return vec![tmux[1].into(), tmux[3].into(), tmux[2].into(), "tmux".into()];
    }

    vec![title.into()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_to_path() {
        assert_eq!(
            title_to_path("Where in the World: Tenaya and Climate Change - qutebrowser"),
            vec!["Where in the World: Tenaya and Climate Change".to_string(), "qutebrowser".to_string()],
        );
        assert_eq!(
            title_to_path("[23%] Where in the World: Tenaya and Climate Change - qutebrowser"),
            vec!["Where in the World: Tenaya and Climate Change".to_string(), "qutebrowser".to_string()],
        );
        assert_eq!(
            title_to_path("[0%] Where in the World: Tenaya and Climate Change - qutebrowser"),
            vec!["Where in the World: Tenaya and Climate Change".to_string(), "qutebrowser".to_string()],
        );
        assert_eq!(
            title_to_path("(•) Rocket.Chat - qutebrowser"),
            vec!["Rocket.Chat".to_string(), "Blockstream".to_string()],
        );
        assert_eq!(
            title_to_path("Rocket.Chat - qutebrowser"),
            vec!["Rocket.Chat".to_string(), "Blockstream".to_string()],
        );
        assert_eq!(
            title_to_path("Inbox (1) - apoelstra@blockstream.com - Blockstream Mail - qutebrowser"),
            vec!["Gmail".to_string(), "Blockstream".to_string()],
        );
        assert_eq!(
            title_to_path("Inbox (10) - apoelstra@blockstream.com - Blockstream Mail - qutebrowser"),
            vec!["Gmail".to_string(), "Blockstream".to_string()],
        );
        assert_eq!(
            title_to_path("Blockstream - Calendar - Tuesday, December 13, 2022, today - qutebrowser"),
            vec!["Calendar".to_string(), "Blockstream".to_string()],
        );
        assert_eq!(
            title_to_path("[mosh] urxvt (camus) - ../check-pr.sh pr/1467/head 1467 (tmux:work-rust-bitcoin/rust-bitcoin)"),
            vec![
                "[mosh] urxvt (camus) - ../check-pr.sh pr/1467/head 1467",
                "rust-bitcoin",
                "work-rust-bitcoin",
                "tmux",
            ],
        );
        assert_eq!(
            title_to_path("Notifications - qutebrowser"),
            vec!["Notifications".to_string(), "Github".to_string()],
        );
        assert_eq!(
            title_to_path("Standardize derives on error types by tcharding · Pull Request #1466 · rust-bitcoin/rust-bitcoin - qutebrowser"),
            vec![
                "#1466 Standardize derives on error types by tcharding".to_string(),
                "Pull Request".to_string(),
                "rust-bitcoin/rust-bitcoin".to_string(),
                "Github".to_string(),
            ],
        );
        assert_eq!(
            title_to_path("TapTweak API for a single script path spending case · Issue #1393 · rust-bitcoin/rust-bitcoin - qutebrowser"),
            vec![
                "#1393 TapTweak API for a single script path spending case".to_string(),
                "Issue".to_string(),
                "rust-bitcoin/rust-bitcoin".to_string(),
                "Github".to_string(),
            ],
        );
        assert_eq!(
            title_to_path("Add Coin Selection Algos · Discussion #1402 · rust-bitcoin/rust-bitcoin - qutebrowser"),
            vec![
                "#1402 Add Coin Selection Algos".to_string(),
                "Discussion".to_string(),
                "rust-bitcoin/rust-bitcoin".to_string(),
                "Github".to_string(),
            ],
        );
        assert_eq!(
            title_to_path("[0%] Add Coin Selection Algos · Discussion #1402 · rust-bitcoin/rust-bitcoin - qutebrowser"),
            vec![
                "#1402 Add Coin Selection Algos".to_string(),
                "Discussion".to_string(),
                "rust-bitcoin/rust-bitcoin".to_string(),
                "Github".to_string(),
            ],
        );
    }
}


