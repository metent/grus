# grus

`grus` is a command-line hierarchial task manager. This means that instead of organizing tasks as a long, unmaintainable list, tasks can be organized in a hierarchy. This allows tasks which may seem unfeasible to be repeatedly broken down into smaller, more doable tasks, making it easier to plan for the long term. `grus` builds onto this idea by representing tasks as nodes of a multitree, which allows multiple tasks to have a same subtask as a child, since two different tasks can depend on the same subtask.

Task management should be quick and should not require much thought. `grus` helps the user by listing only what it considers the most important tasks, so that the user can narrow down their choice instead of getting overwhelmed by tasks. This makes scrolling for tasks mostly unnecessary. It has vi-like key bindings, so most operations: adding, deleting, renaming, take the least amount of keystrokes to perform.

> **Warning**
>
> `grus` is currently undergoing early stages of development. It is unfinished and unstable. Storage format might change between releases, so task storage may be incompatible between releases.

## Features

- Quick workflow
- BFS-based view of tasks inspired by [broot](https://github.com/Canop/broot)
- Multiple tasks can have the same child task, inspired by [grit](https://github.com/climech/grit)
- vi-like key bindings
- Light on resources

## Installation

### From crates.io

```
cargo install grus
```

Make sure to include `$HOME/.cargo/bin` in the `PATH` variable.

## Keybindings

### Tree View

|         Key          | Action                                                                                             |
|         ---          | ---                                                                                                |
|  k or <kbd>up</kbd>  | Move cursor up                                                                                     |
| j or <kbd>down</kbd> | Move cursor down                                                                                   |
|          l           | Make selected task the root task                                                                   |
|          h           | Make previously selected task up the heirarchy, the root task                                      |
|          v           | View all sessions of the current task                                                              |
|          a           | Add a subtask of the selected task with given name                                                 |
|          d           | Delete the selected task and all of its descendents                                                |
|          r           | Rename the selected task                                                                           |
|          z           | Add a due date to the selected task                                                                |
|          Z           | Unset due date of the selected task                                                                |
|          s           | Add a session to the selected task                                                                 |
|          K           | Increase the relative priority of current task among siblings                                      |
|          J           | Decrease the relative priority of current task among siblings                                      |
|        space         | Select the current task                                                                            |
|          x           | Make the selected tasks children of current task while detaching it from the previous parent       |
|          .           | Make the selected tasks children of current task while retaining its link with the previous parent |
|          q           | Quit grus                                                                                          |
|          2           | Switch to session view                                                                             |
|          I           | Import database from ~/sync/tasks                                                                  |
|          E           | Export database to ~/sync/tasks                                                                    |

### Session View

|         Key          | Action                 |
|         ---          | ---                    |
|  k or <kbd>up</kbd>  | Move cursor up         |
| j or <kbd>down</kbd> | Move cursor down       |
|          v           | Toggle sub-mode        |
|          d           | Delete current session |
|          q           | Quit grus              |
|          1           | Switch to tree view    |

## Roadmap

- [x] Basic todo functionality
- [ ] Task sorting by score
- [ ] Decorations
- [ ] Task cut/yank and paste
- [ ] Fuzzy search tasks
- [ ] Notifications
