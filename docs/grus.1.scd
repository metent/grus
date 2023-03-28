grus(1)

# NAME

grus - A multitree-based hierarchical task manager

# SYPNOSIS

grus

# DESCRIPTION

grus is a command-line hierarchial task manager. This means that instead of organizing tasks as a long, unmaintainable list, tasks can be organized in a hierarchy. This allows tasks which may seem unfeasible to be repeatedly broken down into smaller, more doable tasks, making it easier to plan for the long term. grus builds onto this idea by representing tasks as nodes of a multitree, which allows multiple tasks to have a same subtask as a child, since two different tasks can depend on the same subtask.

Task management should be quick and should not require much thought. grus helps the user by listing only what it considers the most important tasks, so that the user can narrow down their choice instead of getting overwhelmed by tasks. This makes scrolling for tasks mostly unnecessary. It has vi-like key bindings, so most operations: adding, deleting, renaming, take the least amount of keystrokes to perform.

# CONCEPTS

## View

Tasks can be displayed to the user in different ways, called views, depending on the requirement of the user. There are currently 2 types of views:

- Tree View
- Session View

## Due Date

Each task can have at most a single due date which marks the deadline of completing the particular task. Due dates can be assigned to the selected task(s) in the tree view using the key z. On pressing this key, the last line prompt enters, which asks the user for a due date. Due dates can be entered in the following formats:

- <date> <time>
- <date>
- <time>

where <date> is replaced by any of the following (all are case-insensitive):

- today
- tomorrow or tmrw
- monday or mon
- tuesday or tue
- wednesday or wed
- thursday or thu
- friday or fri
- saturday or sat
- sunday or sun
- dd/mm/yyyy format like 12/05/2023

and <time> is replaced by 12-hour clock time like 12:03 PM or 4 am.

## Session

Each task can be scheduled any number of times using sessions. Each session has a start date and end date. Sessions can be added to the selected task(s) in the tree view using the key s. On pressing this key, the last line prompt enters, which asks the user for a session. Sessions can be entered in the following formats:

- <date> <time> to <date> <time>
- <date> <time> to <time>
- <time> to <date> <time>
- <time> to <time>

where <date> and <time> are replaced according to the rules discussed in the Due Date section.

# TREE VIEW

When grus is launched, Tree View is the first view which is presented. The task hierarchy is displayed on the screen, along with their due date and earliest session. Not all tasks are displayed however, since we are limited by the height of the terminal. The tasks which are too deep in the heirarchy or tasks with low-priority are not displayed if the height of the terminal is small enough. In precise terms, grus uses a breadth-first search algorithm to display the task tree. To view the hidden tasks, one can move deeper into the hierarchy by moving the cursor over a task and pressing the l button. Doing so will change the root of the tree view to the highlighted task. Now, since only this task and its descendents are displayed, more space is available to display the hidden tasks. To move back to the previous root, one can press the h button.

Depending on the 'order' of subtasks of a task, grus assigns a priority to each task. The priority is indicated by the color of the bullet adjacent to the task name. The redness of the bullet represents higher priority and greenness represents lower priority. Red indicates high priority, Yellow indicates medium priority, green indicates low priority and so on.

When a task is a subtask of two or more tasks, the tree line representing the connection between the subtask and each parent task becomes thicker and has the same color.

# SESSION VIEW

Session View views all the sessions assigned to every task in chronological order of the start date, along with the task to which they are assigned. It also allows deletion of sessions.

# KEYBINDINGS

## Tree View

*k or up*
	Move cursor up

*j or down*
	Move cursor down

*l or right*
	Make selected task the root task

*h or left*
	Make previously selected task up the heirarchy, the root task

*v*
	View all sessions of the current task

*a*
	Add a subtask of the selected task with given name

*D*
	Delete the selected task and all of its descendents

*r*
	Rename the selected task

*z*
	Add a due date to the selected task

*Z*
	Unset due date of the selected task

*s*
	Add a session to the selected task

*K*
	Increase the relative priority of current task among siblings

*J*
	Decrease the relative priority of current task among siblings

*s*
	Select the current task

*x*
	Make the selected tasks children of current task while detaching it from the previous parent

*.*
	Make the selected tasks children of current task while retaining its link with the previous parent

*q*
	Quit grus

*2*
	Switch to session view

*I*
	Import database from ~/sync/tasks

*E*
	Export database to ~/sync/tasks

## Session View

*k or up*
	Move cursor up

*j or down*
	Move cursor down

*v*
	Toggle sub-mode

*D*
	Delete current session

*q*
	Quit grus

*1*
	Switch to tree view
