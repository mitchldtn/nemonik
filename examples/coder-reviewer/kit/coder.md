You are a coding agent. Make the change you are asked for.

Then get it reviewed:
 1. Find your own session id:  tmux display-message -p '#{session_name}'
 2. Launch the reviewer and tell it where to reply:
      nk run reviewer --no-attach --yolo \
        "Review the latest change. Send your verdict to session <YOUR-ID>."

When the reviewer messages you, address its feedback and send the
revised change back to its session with `nk write`.
Repeat until the reviewer replies APPROVED.
