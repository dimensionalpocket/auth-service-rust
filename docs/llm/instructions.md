Follow these instructions for all your tasks:

- Before starting any task, read the `README.md` to get context and progress of the entire project.

- When you need to get the current date or time, run `date +%Y-%m-%d@%H:%M` to get the date and time (without seconds) in `YYYY-MM-DD@HH:MM` format. NEVER guess the date or time. The hour is always in 24-hour format. NEVER use AM or PM.

## Working with Plans

- When you're asked to write a plan:
  - Save the plan in the `docs/llm/plans` directory. If the directory doesn't exist, create it.
  - Use a directory structure based on the current year and month (`YYYY/MM` format). Name the file with the day only (`DD`), followed by a dash, and then the task name in lowercase with dashes. For example: `2025/04/16-do-something.md`. There's no need to add a `-plan` suffix to the file name.
  - Do not start coding immediately after writing the plan. Instead, wait for the plan to be reviewed and approved by the user.
  - If your plan involves implementation, include a summary of files that will be created or modified, and code samples for any new functions or methods. The user is an experienced developer but new to Rust, so he will use the code samples to review your plan.

- When you're asked to proceed with the implementation of a plan, and during the implementation you have to deviate from the plan, you must STOP and inform the user about the deviation in chat, offering options to proceed for the user to choose from.
  - When you're implementing a plan, do not implement anything that is not in the plan.

## Running Commands

- When running Rust commands (`cargo`, `rustc`, etc), always prepend the command with `mise exec --` to ensure the correct environment is used. Example: `mise exec -- cargo test`.
