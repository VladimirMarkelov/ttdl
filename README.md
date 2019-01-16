# TTDL (Terminal ToDo List)

![](https://travis-ci.com/VladimirMarkelov/ttdl.svg?branch=master)
[![](https://img.shields.io/crates/v/ttdl.svg)](https://crates.io/crates/ttdl)
[![Downloads](https://img.shields.io/crates/d/ttdl.svg)](https://crates.io/crates/ttdl)

A CLI tool to manage todo lists in (todo.txt format)[http://todotxt.org/]. A short demo of TTDL in action:

<img src="./images/ttdl_demo.gif" alt="TTDL in action">

## Installation

The application can be compiled from source, or installed using cargo:

```shell
$ cargo install ttdl
```

You need Rust compiler that supports Rust 2018 edition (Rust 1.31 or newer) to do it. If you want to upgrade existing ttdl execute the following command:

```shell
$ cargo install ttdl --force
```

### Precompiled binaries

For Windows and Ubuntu you can download precompiled binaries from (Release page)[https://github.com/VladimirMarkelov/ttdl/releases].

* Windows binary works on Windows 7 or newer Windows.
* Ubuntu binary tested on Ubuntu 16 but should work on Ubuntu 18 (and maybe on other deb-based Linux distributions)

## Known issues

**Adding a new todo, append or prepend a text to existing todo results in error:**

It may happend if the text starts with(or contains only) a project or a context:

```shell
$ ttdl add "+myproject"
Subject is empty
```

**Workaround**: add a space between quotation mark and '+' or '@' symbol. The todo will be added without leading space:

```shell
$ ttdl add " +myproject"
Added todo:
# D P Created    Finished   Due        Threshold  Subject
----------------------------------------------------------
8                                                 +myproject
```

## Configuration

TTDL is a standalone binary and it does not create any files in user's directory. But at start, it checks for a configuration file - please see example configuration (ttdl.toml)[./ttdl.toml] in user's configuration directory and loads it. Local configuration files are supported as well. Locations where TTDL looks for a configuration file:

* current working directory
* Linux:  ~/.config/ttdl.toml
* Windows: c:\Users\{username}\AppData\Roaming\ttdl.toml
* OSX: /Users/{username}/Library/Preferences/ttdl.toml

First, TTDL looks for a configuration file in the current working directory. And only if it does not contain ttdl.toml, the application looks for its configuration file in user's directory.

The configuration file contains options that cannot be set in command line:

- colors for special kinds of todos: overdue, due today, due soon, top priority, high priority, and completed
- ranges for cases "due soon" and "high priority". By default both option are disabled. To enable "due soon", set it to the number of days, so todos that are due in equal to or less than that number(except overdue and due today todos) will be displayed with `soon` color. To enable high priority highlight, set `important` to a priority - all todos with this priority or higher(except top priority ones) will be displayed with `important` color.
- `filename` - the path to global todo file (can point to directory, TTDL adds `todo.txt` automatically if `filename` is a directory). To override the option, you can set environment variable `TTDL_FILENAME` or use command line option `--local` if you need to load todo list from current working directory
- `creation_date_auto` The option defines TTDL behavior when a new todo is added. By default, TTDL adds a todo as is - a user must set manually creation date in the subject. Setting `creation_date_auto` to `true` makes TTDL to set today as creation date for all new todos if their subject does not include creation date.

## How to use

Run TTDL with the command line:

```
ttdl [command] [ID range] [subject] [filter options] [extra options]
```

Options can be at any position. ID range and subject are optional but if you are going to use both, ID range must go first.

If a non-option starts with `+` or `@`, the option is considered as a filter by project or context respectively. Command line can contain as many projects and contexts as needed. In this case, all items of the same group are combined with OR. Example: if you execute `ttdl list +myproj +demo @docs`, it displays all todos that belongs to either `myproj` or `demo`, and have `docs` context.

If the first non-option word contains only digits and dash character, it is treated as a single ID(only digits) or ID range(digits with a dash) or ID list(numbers separated with comma). ID is 1-based: a number between 1 and the number of todos in the entire list. It is OK to use ID out of that range: IDs that greater than the number of todos or lower than 1 are skipped. So, e.g, if you want to remove all todos starting from 10th todo, you can run the command `ttdl remove 10-999999 -a` - `-a` to delete both completed and incomplete todos.

The second non-option(or the first one if ID range is not defined) is a subject. Subject's usage depends on command:

* `add` - it is an entire text for a new todo (including projects, contexts, due date, recurrence);
* `edit` - it is an entire new subject for the first selected todo;
* for the rest commands it can be either a substring(case-insensitive search) or a regular expression to search in the todo's subject, projects, and contexts - depends on the option `--regex`.

NOTES:
1. All dates are entered and displayed in format - YYYY-MM-DD(4 year digits - 2 month digits - 2 day digits)
2. Recurrence is defined in format 'the number of intervals' + 'interval type' without a space between them. Interval type is one of `d` - days, `w` - weeks, `m` - months, `y` - years. Example: to add a todo with your friend's birthday(;et's assume it is 10 of April) use the following command `ttdl add "best friend birthday due:2019-04-10 rec:1y"`. After the birthday passes, just execute `ttdl done <todo-ID>` and it will change its due date to 2020-04-10
3. Recurrence special case: if you set due date to the last day of a month and interval is a month or longer then the next due date will always be the end of the months. Example: a todo `pay credit due:2019-02-28 rec:1m` after executing `ttdl done ID` turns into `pay credit due:2019-03-31`

### Output example

```
# D P Created    Finished   Due        Threshold  Subject
----------------------------------------------------
1 x A 2016-04-30 2016-05-20                       measure space for +chapelShelving @chapel
2   C 2016-05-20                                  paint +chapelShelving @shelve
3 R                         2018-11-11            pay credit card rec:1m
----------------------------------------------------
3 todos (of 3 total)
```

Columns:

* `#` - order number of a todo
* `D` - 'Done', it is empty for an incomplete regular todo, 'x' for a completed todo, and 'R' for recurrent todo
* `P` - priority (from A to Z, empty value means no priority)

### Archive

In the long run e a todo list gets full of completed tasks. They may slow down the todo list management. If you do not need to keep completed stuff, you can delete them using command `remove`. But if completed tasks must be kept for a while, you can archive them. Execute `clean`(or `archive`) command and completed tasks will be moved from the actual todo list to its archive.

Archiving completed todos makes the actual todo list loading faster. Though it has a few drawbacks:

- archived todos cannot be modified (e.g, if you want to delete some archived todos, you have to do it manually in any text editor)
- there is no way to show actual and archived todos at the same

#### How to show archived todos

To display archived todos, use option `--done`. The option enables "archive" mode: the only available command in this mode is `list` and TTDL loads `done.txt` instead of `todo.txt`. On entering this mode, the option `-A` is enabled automatically if neither `-a` nor `-A` is defined.

### Supported commands

The list of available command is short but the commands are powerful. All commands support group operations and dry run mode. Except `add` command that adds a new todo one at a time. Please, refer to section "Examples", it provides a handful of useful examples of how to filter and modify todo list.

Commands:

* add - add a new todo;
* list - show list of todo items. By default it displays all incomplete todos;
* done - mark selected todos completed. If a todo is recurrent its due date moves to the next date but the todo remains incomplete;
* undone - remove `finished` mark from completed todos;
* remove - deletes the selected todos;
* clean  - moves completed todos from main file to `done.txt`. The file `done.txt` is created(if it does not exist) in the same directory where main todo list file is located;
* edit - modify one or few properties for the selected todos. One exception: modifying todo's subject changes only the first selected todo, others are skipped;
* append - adds a text to the end of the selected todos (space between old text and new one is added automatically);
* prepend - inserts a new text at the beginning of the selected todos (space between old text and new one is added automatically).

Most of the commands can be abbreviated. Please refer to built-in TTDL help to get a list of full command names and their aliases.

NOTE: `done` moves a recurrent todo's due date to the next one, but it does not check if the new due date is in the future (it is by design). So, if a monthly task is 2 months overdue, you have to execute `ttdl done ID` two times to push it to the incoming month or manually set a new due date with the command `ttdl edit ID --set-due=YYYY-MM-DD`.

### Tags

The orginal todo.txt format describes a user-defined tags that can be used by any application for special needs. The format of a tag is `tag_name:tag_value`. The original format does not specify any tag - all are considered custom ones.

TTDL supports a few custom tags (as of version 0.3):

* `due` - a todo's due date. The tag value is in format YYYY-MM-DD;
* `t` - a todo's theshold date. The tag value is in format YYYY-MM-DD;
* `rec` - makes a todo recurrent. It makes sense only when using along with `due` tag. The tag value is the number of time intervals and one-character time interval name: `d` - every few days, `w` every few weeks, `m` - every few months, `y` - every few years. Examples: `1w` - a weekly todo, `5d` - every 5 days.

### Extra features

Each command that modifies todo list supports dry run mode. The mode is enabled with an option `--dry-run`. When executing `ttdl` with the option, it finds out which todos would be changed after the command completes, then displays existing todos and their new values.

By default TTDL outputs the todo list in long mode and uses colors. To disable colors, use an option `--no-colors`. To make the output shorter, use an option `--short` to show only a few the most important fields(ID, comlpetion mark, priority, and subject), or choose which fields to display with an option `--fields`: a comma-separated list of fields. 
**NOTE**: the option `--fields` defines only a field visibility, but not its order. So, `--fields=pri,due` and `--fields=due,pri` result in the same output.

For easier reading due date, there is an option `--human` that turns dates into relative dates. So, due date 2018-11-11 can turn into `in 3d` (if the current date is 2018-11-08) or into `3d overdue`(if the current date is 2018-11-14). Using an option `--compact` makes the output even shorter: it removes all `in`s and `overdue`s. To understand whether a todo is overdue or not, just check its color: overdue ones are drawn in red color(unless you used the option `--no-colors` or modified color in TTDL config). Option `--human` supports a list of fields to show as relative ones: `ttdl l --human="due"`.

## Command line examples

By default todos from a given range are processed only if they are incomplete. To process all(both done and incomplete), add an option `--all` or `-a`. To process only done todos, add an option `-A`. NOTE: the only exception is the command `clean`|`archive`, it enables option `-A` automatically if `--all` is not present in command line.

### List and filter

| Command | Description |
|---|---|
| `ttdl l 2` | show a single todo with ID 2 |
| `ttdl l 2-5` | show todos with ID from 2 through 5 |
| `ttdl l 2,5` | show only todos with ID 2 and 5 |
| `ttdl l -s=proj,pri` | show all todos sorted by their project and by priority inside each project |
| `ttdl l "car*"` | list todos which have substring `car*` in their subject, project or context |
| `ttdl l "car*" -e` | list todos which have subject, project or context matched regular expression `car*` |
| `ttdl l "car"` | list todos which have substring `car` in their subject, project or context |
| `ttdl l --pri=a` | show todos with the highest priority A |
| `ttdl l --pri=b+` | show todos with priority B and higher (only A and B in this case) |
| `ttdl l +car +train` | show todos which related either to `car` or to `train` projects |
| `ttdl l +my* @*tax` | show todos that have a project tag starts with `my` and a context ends with `tax` |
| `ttdl l --due=tomorrow` | show todos that are due tomorrow |
| `ttdl l --due=soon` | show todos which are due are due in less a few days, including overdue ones (the range is configurable and default value is 7 days) |
| `ttdl l --due=overdue` | show overdue todos |
| `ttdl l --due=today` | show todos that are due today |
| `ttdl l +myproj @ui @rest` | show todos related to project 'myproj' which contains either 'ui' or 'rest' context |

### Add a new todo

| Command | Description |
|---|---|
| `ttdl a "send tax declaration +personal @finance @tax due:2018-04-01 rec:1y"` | add a new recurrent todo(yearly todo) with a due date first of April every year |

### Done (undone)

| Command | Description |
|---|---|
| `ttdl d 2-5` | mark todos with IDs from 2 through 5 done |

### Clean up the list

| Command | Description |
|---|---|
| `ttdl rm 2-5` | delete incomplete todos with IDs from 2 thorough 5 |
| `ttdl rm 2-5 -a` | delete both done and incomplete todos with IDs from 2 through 5 |
| `ttdl rm 2-5 -A` | delete all done todos with IDs from 2 through 5 |
| `ttdl clean 2-5 --wipe` | delete all completed todos with IDs from 2 through 5. It does the same as the previous command does |
| `ttdl clean 2-5` | move all completed todos with IDs from 2 through 5 to done.txt |

### Modify todo list

| Command | Description |
|---|---|
| `ttdl e 2-5 "new subject"` | only the first incomplete todo with ID between 2 and 5 changes its subject (in this case todo with ID equals 2 gets subject "new subject") |
| `ttdl e +proj --repl-ctx=bug1010@bug1020` | replace context `bug1010` with `bug1020` for all incomplete todos that related to project `proj` |
| `ttdl e @customer_acme --set-due=2018-12-31` | set due date 2018-12-31 for all incomplete todos that has `customer_acme` context |
| `ttdl e @customer_acme --set-due=none` | remove due date 2018-12-31 for all incomplete todos that has `customer_acme` context |
| `ttdl e --pri=none --set-pri=z` | set the lowest priority for all incomplete todos which do not have a priority set |
| `ttdl e @bug1000 --set-pri=+` | increase priority for all incomplete todos which have context `bug1000`, todos which did not have priority set get the lowest priority `z` |
