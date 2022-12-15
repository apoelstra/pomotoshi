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

//! Colors
//!
//! Color fading support
//!

pub fn fade_between(
    initial_col: (u8, u8, u8),
    final_col: (u8, u8, u8),
    duration: std::time::Duration,
    total_duration: std::time::Duration,
) -> String {
    let lam = (duration.as_micros() as f64) / (total_duration.as_micros() as f64);

    let lam = 1.0 - (1.0 - lam).powi(2); // make fade more extreme near the start

    let blend_r = (initial_col.0 as f64) * (1.0 - lam) + (final_col.0 as f64) * lam;
    let blend_g = (initial_col.1 as f64) * (1.0 - lam) + (final_col.1 as f64) * lam;
    let blend_b = (initial_col.2 as f64) * (1.0 - lam) + (final_col.2 as f64) * lam;
    format!(
        "#{:02x}{:02x}{:02x}",
        blend_r as u8, blend_g as u8, blend_b as u8
    )
}
