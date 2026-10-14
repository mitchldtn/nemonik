//! A tmux control-mode (`tmux -C`) client. Framing, as observed on tmux 3.7b:
//! - attaching emits an empty `%begin`/`%end` handshake; a command's reply is the second block;
//! - notifications (`%output`, ...) are top-level lines between blocks;
//! - `%output` escapes non-printable bytes as `\ooo` octal (backslashes too, per tmux's docs);
//! - a failed attach sends one `%begin`/`%error` block (no handshake), then `%exit`.

use anyhow::Context;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;

/// One parsed control-mode line; everything this module doesn't need is `Other`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ControlModeEvent {
    CommandOutput { number: u64, lines: Vec<String> },
    CommandError { number: u64, lines: Vec<String> },
    Output { pane_id: String, text: String },
    Exit,
    Other { raw: String },
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Idle,
    InBlock { number: u64, lines: Vec<String> },
}

#[cfg(test)]
fn parse_transcript(lines: &[&str]) -> Vec<ControlModeEvent> {
    let mut state = State::Idle;
    let mut events = Vec::new();
    for &line in lines {
        state = step(state, line, &mut events);
    }
    events
}

fn step(state: State, line: &str, events: &mut Vec<ControlModeEvent>) -> State {
    if let Some((pane_id, text)) = parse_output_line(line) {
        events.push(ControlModeEvent::Output { pane_id, text });
        return state;
    }
    if is_exit_line(line) {
        events.push(ControlModeEvent::Exit);
        return state;
    }
    match state {
        State::Idle => {
            if let Some(number) = parse_framing(line, "%begin") {
                State::InBlock {
                    number,
                    lines: Vec::new(),
                }
            } else if let Some(raw) = line.strip_prefix('%') {
                events.push(ControlModeEvent::Other {
                    raw: raw.to_string(),
                });
                State::Idle
            } else {
                // Unexpected outside a block; surface it rather than drop it.
                events.push(ControlModeEvent::Other {
                    raw: line.to_string(),
                });
                State::Idle
            }
        }
        State::InBlock { number, mut lines } => {
            if let Some(end_number) = parse_framing(line, "%end") {
                if end_number == number {
                    events.push(ControlModeEvent::CommandOutput { number, lines });
                    return State::Idle;
                }
                lines.push(line.to_string());
                State::InBlock { number, lines }
            } else if let Some(err_number) = parse_framing(line, "%error") {
                if err_number == number {
                    events.push(ControlModeEvent::CommandError { number, lines });
                    return State::Idle;
                }
                lines.push(line.to_string());
                State::InBlock { number, lines }
            } else {
                // Pane content can start with `%`; only the framing keywords are special.
                lines.push(line.to_string());
                State::InBlock { number, lines }
            }
        }
    }
}

/// Parses `%begin`/`%end`/`%error <timestamp> <number> <flags>`; `number` pairs a
/// block's open and close.
fn parse_framing(line: &str, keyword: &str) -> Option<u64> {
    let rest = line.strip_prefix(keyword)?.strip_prefix(' ')?;
    let mut parts = rest.split_whitespace();
    let _timestamp = parts.next()?;
    parts.next()?.parse::<u64>().ok()
}

fn parse_output_line(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("%output ")?;
    let (pane_id, escaped_text) = rest.split_once(' ')?;
    Some((pane_id.to_string(), unescape_octal(escaped_text)))
}

fn is_exit_line(line: &str) -> bool {
    line == "%exit" || line.starts_with("%exit ")
}

/// Reverses `\ooo` octal escaping, on bytes (the payload may not be UTF-8).
fn unescape_octal(escaped: &str) -> String {
    let bytes = escaped.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let is_octal_escape = bytes[i] == b'\\'
            && i + 4 <= bytes.len()
            && bytes[i + 1..i + 4]
                .iter()
                .all(|b| (b'0'..=b'7').contains(b));
        if is_octal_escape {
            let octal =
                std::str::from_utf8(&bytes[i + 1..i + 4]).expect("ASCII digits are valid UTF-8");
            out.push(u8::from_str_radix(octal, 8).expect("validated 3-digit octal"));
            i += 4;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CommandReply {
    Output(Vec<String>),
    Error(Vec<String>),
}

fn take_stdout(child: &mut Child) -> anyhow::Result<ChildStdout> {
    child.stdout.take().context("tmux -C child has no stdout")
}

fn write_command(child: &mut Child, command: &str) -> anyhow::Result<()> {
    let stdin = child.stdin.as_mut().context("tmux -C child has no stdin")?;
    writeln!(stdin, "{command}").context("failed to write command to tmux -C stdin")?;
    stdin.flush().context("failed to flush tmux -C stdin")
}

/// Folds tmux's stderr into `err`. Must run only after the child is killed and reaped:
/// reading a live child's stderr blocks forever on a wedged tmux.
fn attach_stderr_context(child: &mut Child, err: anyhow::Error) -> anyhow::Error {
    let stderr_text = child
        .stderr
        .take()
        .map(|mut s| {
            let mut buf = String::new();
            let _ = s.read_to_string(&mut buf);
            buf
        })
        .unwrap_or_default();
    let stderr_text = stderr_text.trim();
    if stderr_text.is_empty() {
        err
    } else {
        err.context(format!("tmux -C stderr: {stderr_text}"))
    }
}

/// `BufRead::lines()` trimming without consuming the reader, so `stream_session` can
/// keep reading.
fn strip_line_ending(line: &str) -> &str {
    line.strip_suffix('\n')
        .map(|s| s.strip_suffix('\r').unwrap_or(s))
        .unwrap_or(line)
}

/// Reads frames until the second block closes, returning the parser state so a
/// streaming caller can resume from it.
fn read_until_second_block<R: BufRead>(reader: &mut R) -> (State, anyhow::Result<CommandReply>) {
    let mut state = State::Idle;
    let mut blocks_seen = 0u32;
    let mut last_block: Option<CommandReply> = None;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = match reader.read_line(&mut line) {
            Ok(n) => n,
            Err(e) => {
                return (
                    state,
                    Err(anyhow::Error::new(e).context("failed to read tmux -C output")),
                );
            }
        };
        if bytes_read == 0 {
            return (state, Err(exit_before_second_block_error(last_block)));
        }
        let trimmed = strip_line_ending(&line);
        let mut events = Vec::new();
        state = step(state, trimmed, &mut events);
        for event in events {
            match event {
                ControlModeEvent::CommandOutput { lines, .. } => {
                    blocks_seen += 1;
                    if blocks_seen == 2 {
                        return (state, Ok(CommandReply::Output(lines)));
                    }
                    last_block = Some(CommandReply::Output(lines));
                }
                ControlModeEvent::CommandError { lines, .. } => {
                    blocks_seen += 1;
                    if blocks_seen == 2 {
                        return (state, Ok(CommandReply::Error(lines)));
                    }
                    last_block = Some(CommandReply::Error(lines));
                }
                ControlModeEvent::Exit => {
                    return (state, Err(exit_before_second_block_error(last_block)));
                }
                ControlModeEvent::Output { .. } | ControlModeEvent::Other { .. } => {}
            }
        }
    }
}

/// Surfaces the incomplete block's content (e.g. "can't find session"), not a generic
/// message.
fn exit_before_second_block_error(last_block: Option<CommandReply>) -> anyhow::Error {
    match last_block {
        Some(CommandReply::Error(lines)) => {
            anyhow::anyhow!(
                "tmux -C reported an error before a second block arrived: {}",
                lines.join(" / ")
            )
        }
        Some(CommandReply::Output(lines)) => anyhow::anyhow!(
            "tmux -C exited after only one block (expected a handshake block plus the command's reply): {}",
            lines.join(" / ")
        ),
        None => anyhow::anyhow!("tmux -C exited before any block was received"),
    }
}

/// One event from a `stream_session` attach.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StreamEvent {
    /// The full screen, captured right after attaching (`%output` only reports new bytes).
    InitialSnapshot(Vec<String>),
    /// A live `%output` event as it happens, already unescaped.
    Output { pane_id: String, text: String },
    /// `%exit` or EOF, including the caller stopping it. Not an error.
    Ended,
    /// The initial `capture-pane -p` got `%error`, or reading failed.
    Failed(String),
}

/// Owns the attach's `Child` and kills and reaps it on drop: `Child` doesn't reap
/// on its own `Drop`.
pub(crate) struct StreamHandle {
    child: Child,
}

impl StreamHandle {
    /// Ends the watch now; the later `Drop` is a harmless no-op.
    pub(crate) fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Attaches in control mode and streams output live; the caller ends it via the
/// returned `StreamHandle`. No timeout on the initial snapshot, and no batching of
/// `%output` (that's the consumer's policy).
pub(crate) fn stream_session(
    session: &str,
) -> anyhow::Result<(StreamHandle, mpsc::Receiver<StreamEvent>)> {
    let target = nemonik_core::tmux::exact_target(session);
    let mut child = Command::new("tmux")
        .arg("-C")
        .arg("attach-session")
        .arg("-t")
        .arg(&target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn tmux -C attach-session")?;

    // Chained so setup failures still clean up; after that the StreamHandle owns the child.
    let setup_result = take_stdout(&mut child)
        .and_then(|stdout| write_command(&mut child, "capture-pane -p").map(|()| stdout));

    let stdout = match setup_result {
        Ok(stdout) => stdout,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(attach_stderr_context(&mut child, e));
        }
    };

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || stream_output(BufReader::new(stdout), tx));
    Ok((StreamHandle { child }, rx))
}

/// Thread body: sends the initial snapshot, then forwards `%output` until `%exit`/EOF
/// or the receiver is dropped. Never reads stderr: the child may still be alive.
/// `%output` arriving before the snapshot reply is discarded.
fn stream_output<R: BufRead>(mut reader: R, tx: mpsc::Sender<StreamEvent>) {
    let (mut state, snapshot_result) = read_until_second_block(&mut reader);
    match snapshot_result {
        Ok(CommandReply::Output(lines)) => {
            if tx.send(StreamEvent::InitialSnapshot(lines)).is_err() {
                return;
            }
        }
        Ok(CommandReply::Error(lines)) => {
            // An error reply must never be reported as pane content.
            let _ = tx.send(StreamEvent::Failed(format!(
                "tmux -C reported an error on the initial capture-pane: {}",
                lines.join(" / ")
            )));
            return;
        }
        Err(e) => {
            let _ = tx.send(StreamEvent::Failed(format!("{e:#}")));
            return;
        }
    }

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let _ = tx.send(StreamEvent::Ended);
                return;
            }
            Ok(_) => {}
            Err(e) => {
                let _ = tx.send(StreamEvent::Failed(format!(
                    "failed to read tmux -C output: {e}"
                )));
                return;
            }
        }
        let trimmed = strip_line_ending(&line);
        let mut events = Vec::new();
        state = step(state, trimmed, &mut events);
        for event in events {
            match event {
                ControlModeEvent::Output { pane_id, text } => {
                    if tx.send(StreamEvent::Output { pane_id, text }).is_err() {
                        return;
                    }
                }
                ControlModeEvent::Exit => {
                    let _ = tx.send(StreamEvent::Ended);
                    return;
                }
                // No more commands are sent, so later blocks aren't expected; ignore them.
                ControlModeEvent::CommandOutput { .. }
                | ControlModeEvent::CommandError { .. }
                | ControlModeEvent::Other { .. } => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_the_empty_handshake_block_tmux_emits_immediately_on_attach() {
        // Captured live: attach, before any command.
        let lines = ["%begin 1786941587 5658 0", "%end 1786941587 5658 0"];
        assert_eq!(
            parse_transcript(&lines),
            vec![ControlModeEvent::CommandOutput {
                number: 5658,
                lines: vec![]
            }]
        );
    }

    #[test]
    fn parses_a_successful_capture_pane_reply() {
        // Captured live: `capture-pane -p` of a pane showing "hello world" (blank lines trimmed).
        let lines = [
            "%begin 1786941587 5663 1",
            "hello world",
            "",
            "",
            "%end 1786941587 5663 1",
        ];
        assert_eq!(
            parse_transcript(&lines),
            vec![ControlModeEvent::CommandOutput {
                number: 5663,
                lines: vec!["hello world".to_string(), String::new(), String::new()],
            }]
        );
    }

    #[test]
    fn parses_notifications_between_blocks_as_other() {
        // Exact lines captured live, between two blocks.
        let lines = ["%session-changed $2 cmtest", "%window-renamed @2 sleep"];
        assert_eq!(
            parse_transcript(&lines),
            vec![
                ControlModeEvent::Other {
                    raw: "session-changed $2 cmtest".to_string()
                },
                ControlModeEvent::Other {
                    raw: "window-renamed @2 sleep".to_string()
                },
            ]
        );
    }

    #[test]
    fn parses_a_command_error_reply() {
        // Exact lines captured live for a deliberately invalid command.
        let lines = [
            "%begin 1786941623 5713 1",
            "parse error: unknown command: bogus-command-xyz",
            "%error 1786941623 5713 1",
        ];
        assert_eq!(
            parse_transcript(&lines),
            vec![ControlModeEvent::CommandError {
                number: 5713,
                lines: vec!["parse error: unknown command: bogus-command-xyz".to_string()],
            }]
        );
    }

    #[test]
    fn parses_output_notifications_with_octal_unescaping() {
        // Exact line captured live: \015=CR, \012=LF, \033=ESC.
        let lines = ["%output %3 \\015\\012\\033[?2004l\\015"];
        assert_eq!(
            parse_transcript(&lines),
            vec![ControlModeEvent::Output {
                pane_id: "%3".to_string(),
                text: "\r\n\x1b[?2004l\r".to_string(),
            }]
        );
    }

    #[test]
    fn parses_exit_at_the_top_level() {
        let lines = ["%exit"];
        assert_eq!(parse_transcript(&lines), vec![ControlModeEvent::Exit]);
    }

    #[test]
    fn unescape_octal_handles_the_documented_self_escaping_backslash_case() {
        // Per tmux's docs; not observed live.
        assert_eq!(unescape_octal("a\\134b"), "a\\b");
    }

    #[test]
    fn a_line_that_only_looks_like_end_but_has_a_mismatched_number_stays_block_content() {
        // Defensive: mismatched block numbers must not close the wrong block.
        let lines = ["%begin 100 1 0", "%end 100 999 0", "%end 100 1 0"];
        assert_eq!(
            parse_transcript(&lines),
            vec![ControlModeEvent::CommandOutput {
                number: 1,
                lines: vec!["%end 100 999 0".to_string()],
            }]
        );
    }

    #[test]
    fn an_output_notification_interleaved_inside_an_open_block_is_still_surfaced() {
        // Not observed live, but `%output` inside a block is surfaced immediately.
        let lines = ["%begin 1 2 0", "%output %1 hi", "%end 1 2 0"];
        assert_eq!(
            parse_transcript(&lines),
            vec![
                ControlModeEvent::Output {
                    pane_id: "%1".to_string(),
                    text: "hi".to_string()
                },
                ControlModeEvent::CommandOutput {
                    number: 2,
                    lines: vec![]
                },
            ]
        );
    }

    #[test]
    fn read_until_second_block_surfaces_a_command_error_as_a_command_error_not_output() {
        let transcript = "\
%begin 1 100 0
%end 1 100 0
%begin 1 101 1
parse error: unknown command: bogus
%error 1 101 1
";
        let mut reader = BufReader::new(Cursor::new(transcript.as_bytes()));
        let (_, result) = read_until_second_block(&mut reader);
        assert_eq!(
            result.unwrap(),
            CommandReply::Error(vec!["parse error: unknown command: bogus".to_string()])
        );
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("simulated read failure"))
        }
    }

    #[test]
    fn stream_output_reports_failed_not_initial_snapshot_when_capture_pane_errors() {
        let transcript = "\
%begin 1 100 0
%end 1 100 0
%begin 1 101 1
can't find pane
%error 1 101 1
";
        let reader = BufReader::new(Cursor::new(transcript.as_bytes()));
        let (tx, rx) = mpsc::channel();
        stream_output(reader, tx);

        match rx.recv().expect("expected one event") {
            StreamEvent::Failed(message) => assert!(
                message.contains("can't find pane"),
                "expected the error content in the Failed message, got: {message}"
            ),
            other => panic!("expected Failed, got {other:?}"),
        }
        assert!(
            rx.recv().is_err(),
            "expected no further events after Failed"
        );
    }

    #[test]
    fn stream_output_reports_failed_not_ended_on_a_mid_stream_io_error() {
        let valid_snapshot = "\
%begin 1 100 0
%end 1 100 0
%begin 1 101 1
hello
%end 1 101 1
";
        let reader = BufReader::new(Cursor::new(valid_snapshot.as_bytes()).chain(FailingReader));
        let (tx, rx) = mpsc::channel();
        stream_output(reader, tx);

        match rx.recv().expect("expected an initial snapshot event") {
            StreamEvent::InitialSnapshot(lines) => assert_eq!(lines, vec!["hello".to_string()]),
            other => panic!("expected InitialSnapshot, got {other:?}"),
        }
        match rx
            .recv()
            .expect("expected a second event after the snapshot")
        {
            StreamEvent::Failed(message) => {
                assert!(
                    message.contains("simulated read failure"),
                    "expected the I/O error in the message, got: {message}"
                )
            }
            other => panic!(
                "expected Failed, not {other:?} (Ended would silently discard the I/O error)"
            ),
        }
    }
}
