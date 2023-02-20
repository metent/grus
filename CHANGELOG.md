### Unreleased

#### Added

- Tree view now displays division lines.
- Tasks can now be selected in tree view.
- Tasks can now be moved using cut action.
- Selecting multiple tasks and then performing add action creates a shared task.
- Selecting multiple tasks and then performing rename or due date modifiction modifies the name or due date respectively, in all selected tasks.
- Shared children are now indicated using colored tree lines.
- Relative priority of tasks among siblings is now indicated using colored bullets in tree view.
- Cursor can now be moved during editing in last line mode.
- Tasks can now be made children of other tasks without breaking any existing links using the share action.
- Multiple sessions can now be added to tasks using add session action.
- New view: session view. Lists all sessions and the tasks to which they belong, in incresing order of session start time.
- Task sub-mode in session view. Lists sessions of a single task. This sub-mode can be accessed in either tree or session view.

#### Fixed

- Fixed hang on delete action.
- Text now wraps properly when it contains only a single word of width equal to allowed width.

#### Changed

- Rename action now puts the previous name in the last line prompt.
- Subtasks of the same task are now displayed only once in tree view.
- Improved date parsing.

### v0.1.0

First public release
