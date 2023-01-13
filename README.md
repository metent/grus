# grus

`grus` is a command-line hierarchial task manager. Heirarchial task management allows very long tasks to be broken down into smaller and smaller subtasks and hence is also suitable for long-term planning. This allows one to break long-spanning tasks into more manageable short-spanning subtasks.

`grus` focuses on making planning as quick as possible, so that one spends more time working, and less time planning. It fills the screen with the most important tasks, so that the user doesn't get overwhelmed with a lot of tasks. This makes scrolling for tasks mostly unnecessary. It has vi-like key bindings, so most operations: adding, deleting, renaming, take the least amount of keystrokes to perform.

> **Warning**
> `grus` is currently in very early stages of development. It is unfinished and unstable. Storage format might change between releases, so task storage generated by previous releases might be incompatible with the current release.

## Features

- Quick workflow
- BFS-based view of tasks inspired by [broot](https://github.com/Canop/broot)
- vi-like key bindings
- Light on resources

## Installation

### From crates.io

```
cargo install grus
```

Make sure to include `$HOME/.cargo/bin` in the `PATH` variable.

## Keybindings

| Key | Action                                                        |
| --- | ---                                                           |
|  k  | Move selection up                                             |
|  j  | Move selection down                                           |
|  l  | Make selected task the root task                              |
|  h  | Make previously selected task up the heirarchy, the root task |
|  a  | Add a subtask of the selected task with given name            |
|  d  | Delete the selected task and all of its descendents           |
|  r  | Rename the selected task                                      |
|  x  | Add a due date to the selected task                           |
|  X  | Unset due date of the selected task                           |
|  H  | Set priority of the selected task to high                     |
|  M  | Set priority of the selected task to medium                   |
|  L  | Set priority of the selected task to low                      |
|  N  | Unset priority of the selected task                           |

## Roadmap

- [x] Basic todo functionality
- [ ] Task sorting by score
- [ ] Decorations
- [ ] Task cut/yank and paste
- [ ] Fuzzy search tasks
- [ ] Notifications
