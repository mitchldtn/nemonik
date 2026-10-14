---
title: Review a PR with an agent
group: Guides
order: 1
description: Start the code:review kit on a PR in its own window and have it report back to your session.
---

Start the `code:review` kit on a PR in its own window. It posts the review on the PR, then tells your session it is done.

## Start the review

Find your own session's name, then start the reviewer in a new window and tell it where to report back.

```sh title="shell"
$ tmux display-message -p '#{session_name}'
$ nk run mitchldtn/code:review --project my-app --new \
    "review pr 128 and when you are done run nk write <your session> <text>"
```

## Posting

The kit posts the review as a PR comment, in a fixed template: required fixes, recommended improvements, redundant code, test coverage, long-term risks and questions. It asks before posting a possible security finding.

## Another round

When you've addressed the review, message the reviewer's session instead of starting a new one; it still has the PR in context.

```sh title="shell"
$ nk write <review session> "addressed in the latest commit, please review pr 128 again"
```

:::note
Keep these messages short and point to the PR; the details belong in the PR comments.
:::
