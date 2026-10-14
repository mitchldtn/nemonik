//! Remote control: lets the account that started remote control drive or watch a
//! running session via nemonik's server. That is remote code execution over an authenticated channel, so
//! every pushed command is logged to `~/.nemonik/harness-daemon.log`.

mod control_mode;
pub mod daemon;
pub mod spawn;
pub mod watch;
