# Table of Contents

- [TTDL (Terminal ToDo List)](#ttdl-terminal-todo-list)
  - [Installation](#installation)
    - [Precompiled binaries](#precompiled-binaries)
    - [Homebrew](#homebrew)
  - [Known issues](#known-issues)
  - [Configuration](#configuration)
    - [Extra ways to set filenames of active and archived todos](#extra-ways-to-set-filenames-of-active-and-archived-todos)
  - [How to use](#how-to-use)
    - [Output example](#output-example)
    - [Filtering](#filtering)
    - [Archive](#archive)
      - [How to show archived todos](#how-to-show-archived-todos)
    - [Supported commands](#supported-commands)
      - [Calendar](#calendar)
    - [Tags](#tags)
    - [Hashtags](#hashtags)
    - [Time tracking](#time-tracking)
    - [Statistics](#statistics)
    - [Custom formatting](#custom-formatting)
      - [How to enable custom formatting](#how-to-enable-custom-formatting)
      - [Plugin interaction](#plugin-interaction)
      - [Example](#example)
    - [Extra features](#extra-features)
      - [Syntax highlight](#syntax-highlight)
    - [Human-readable dates](#human-readable-dates)
    - [Custom columns](#custom-columns)
      - [Custom column example]($custom-column-example)
  - [Command line examples](#command-line-examples)
    - [List and filter](#list-and-filter)
    - [Add a new todo](#add-a-new-todo)
    - [Done (undone)](#done-undone)
    - [Clean up the list](#clean-up-the-list)
    - [Modify todo list](#modify-todo-list)
    - [Use human-readable dates](#use-human-readable-dates)
    - [Show all project or context tags](#show-all-project-or-context-tags)

## TTDL (Terminal ToDo List)

![build](https://travis-ci.com/VladimirMarkelov/ttdl.svg?branch=master)
[![crates.io](https://img.shields.io/crates/v/ttdl.svg)](https://crates.io/crates/ttdl)
[![Downloads](https://img.shields.io/crates/d/ttdl.svg)](https://crates.io/crates/ttdl)

A CLI tool to manage todo lists in [todo.txt format](http://todotxt.org/). A short demo of TTDL in action:

<img src="./images/ttdl_demo.gif" alt="TTDL in action">

## Installation

The application can be compiled from source, or installed using cargo:

```shell
cargo install ttdl
```

You need Rust compiler that supports Rust 2018 edition (Rust 1.31 or newer) to do it. If you want to upgrade existing ttdl execute the following command:

```shell
cargo install ttdl --force
```

### Precompiled binaries

For Windows and Ubuntu you can download precompiled binaries from [Release page](https://github.com/VladimirMarkelov/ttdl/releases).

- Windows binary works on Windows 7 or newer Windows.
- Ubuntu binary tested on Ubuntu 16 but should work on Ubuntu 18 (and maybe on other deb-based Linux distributions)

### Homebrew

For macOS and Linux you can install TTDL using [Homebrew](https://brew.sh/):

```shell
brew install ttdl
```

## Known issues

**Adding a new todo, append or prepend a text to existing todo results in error:**

It may happen if the text starts with(or contains only) a project or a context:

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

TTDL is a standalone binary and it does not create any files in user's directory. But at start, it checks for a configuration file - please see example configuration [ttdl.toml](./ttdl.toml) in user's configuration directory and loads it. Local configuration files are supported as well. Locations where TTDL looks for a configuration file:

- current working directory
- Linux: `~/.config/ttdl/ttdl.toml`
- Windows: `c:\Users\{username}\AppData\Roaming\ttdl\ttdl.toml`
- OSX: `/Users/{username}/Library/Preferences/ttdl/ttdl.toml`

First, TTDL looks for a configuration file in the current working directory. And only if it does not contain ttdl.toml, the application looks for its configuration file in user's directory. Automatic configuration path detection can be overridden with command line option `-c` or `--config`. If the option is set in command line TTDL disables automatic detection of the configuration file path.

The configuration file contains options that cannot be set in command line:

- default foreground color(usually it is white color for dark-themed terminal). By default, the option is not in the configuration file that means `None` = terminal default color.
- colors for special kinds of todos: overdue, due today, due soon, top priority, high priority, and completed
- ranges for cases "due soon" and "high priority". By default both option are disabled. To enable "due soon", set it to the number of days, so todos that are due in equal to or less than that number(except overdue and due today todos) will be displayed with `soon` color. To enable high priority highlight, set `important` to a priority - all todos with this priority or higher(except top priority ones) will be displayed with `important` color.
- `filename` - the path to global todo file (can point to directory, TTDL adds `todo.txt` automatically if `filename` is a directory). To override the option, you can set environment variable `TTDL_FILENAME` or use command line option `--local` if you need to load todo list from current working directory
- `creation_date_auto` The option defines TTDL behavior when a new todo is added. By default, TTDL adds a todo as is - a user must set manually creation date in the subject. Setting `creation_date_auto` to `true` makes TTDL to set today as creation date for all new todos if their subject does not include creation date.

### Extra ways to set filenames of active and archived todos

Rules to choose a file which is loaded as todo list at startup (from lowest to highest priority):

1. Default is a file "todo.txt" in the current working directory
2. Configuration file option `filename` in section `global`
3. Value of environment variable `TTDL_FILENAME`
4. Command line option `--todo-file`
5. If option `--local` is set it overrides all options above and loads "todo.txt" from the current working directory

If any path at steps 2-5 points to a directory then "todo.txt" is added automatically.

Rules to choose a file which is used to store archived todos (from lowest to highest priority):

1. "done.txt" in the same directory with "todo.txt"
2. Command line option `--done-file`. If any path set in command line points to a directory then "done.txt" is added automatically. If the value is only a filename without directory then the directory is inherited from todo list file

## How to use

Run TTDL with the command line:

```shell
ttdl [command] [ID range] [subject] [filter options] [extra options]
```

Options can be at any position. ID range and subject are optional but if you are going to use both, ID range must go first.

If a non-option starts with `+` or `@`, the option is considered as a filter by project or context respectively. Command line can contain as many projects and contexts as needed. In this case, all items of the same group are combined with OR. Example: if you execute `ttdl list +myproj +demo @docs`, it displays all todos that belongs to either `myproj` or `demo`, and have `docs` context.

If the first non-option word contains only digits and dash character, it is treated as a single ID(only digits) or ID range(digits with a dash) or ID list(numbers separated with comma). ID is 1-based: a number between 1 and the number of todos in the entire list. It is OK to use ID out of that range: IDs that are greater than the number of todos or lower than 1 are skipped. So, e.g., if you want to remove all todos starting from 10th todo, you can run the command `ttdl remove 10-999999 -a` - `-a` to delete both completed and incomplete todos.

The second non-option(or the first one if ID range is not defined) is a subject. Subject's usage depends on command:

- `add` - it is an entire text for a new todo (including projects, contexts, due date, recurrence);
- `edit` - it is an entire new subject for the first selected todo;
- for the rest commands it can be either a substring(case-insensitive search) or a regular expression to search in the todo's subject, projects, and contexts - depends on the option `--regex`.

NOTES:

1. All dates are entered and displayed in format - YYYY-MM-DD(4 year digits - 2 month digits - 2 day digits, or ISO 8601 date format)
2. Recurrence is defined in format 'the number of intervals' + 'interval type' without a space between them. Interval type is one of `d` - days, `w` - weeks, `m` - months, `y` - years. Example: to add a todo with your friend's birthday(let's assume today is 10th of April) use the following command `ttdl add "best friend birthday due:2019-04-10 rec:1y"`. After the birthday passes, just execute `ttdl done <todo-ID>` and it will change its due date to 2020-04-10
3. Recurrence special case: if you set due date to the last day of a month and interval is a month or longer then the next due date will always be the end of the months. Example: a todo `pay credit due:2019-02-28 rec:1m` after executing `ttdl done ID` turns into `pay credit due:2019-03-31`

### Output example

```shell
# D P Created    Finished   Due        Threshold  Subject
----------------------------------------------------
1 x A 2016-04-30 2016-05-20                       measure space for +chapelShelving @chapel
2   C 2016-05-20                                  paint +chapelShelving @shelve
3 R                         2018-11-11            pay credit card rec:1m
----------------------------------------------------
3 todos (of 3 total)
```

Columns:

- `#` - order number of a todo
- `D` - 'Done', it is empty for an incomplete regular todo, 'x' for a completed todo, and 'R' for recurrent todo
- `P` - priority (from A to Z, empty value means no priority)
- `T` - marks an active todo - a todo which has its timer running to track time spent on it

### Filtering

TTDL allows operations on a range of todos. A range is defined either by todo IDs or by todo attribute values.
The filters can be used to limit displayed todos with some condition, or to `edit` or `done` a group of command.
E.g., you can mark `done` all todos of a `project` using a single command.

All examples below are for `list` command, but filter can be used for more commands, like `done` and `edit`.

#### Filter by IDs

If the first argument after a command is a number-like, TTDL treats it as todo ID or todo ID list.
Supported ID formats:

- `ttdl list 5` - a single todo ID, lists a single todo with ID = 5(if it is incomplete)
- `ttdl list 2-4` - a range, show all todos from 2nd to 4th(inclusive)
- `ttdl list 2,4` - a list of IDs, show two todos(if they are incomplete): only 2nd and 4th ones

#### Filter by priority

Priority filter is set with `--pri` argument. Available filter kinds:

- `ttdl list --pri=none` - show all incomplete todos that does not have priority set
- `ttdl list --pri=-` - the same as above
- `ttdl list --pri=any` - show all incomplete todos that have non-empty priority
- `ttdl list --pri=+` - as above
- `ttdl list --pri=#` - where `#` is a low case Latin letter between `a` and `z`, show all incomplete todos that have the same priority
- `ttdl list --pri=#+` - where `#` is a low case Latin letter between `a` and `z`, show all incomplete todos that have priority `#` or higher

#### Filter by recurrence

- `ttdl list --rec=none` - show all non-recurrent todos
- `ttdl list --rec=-` - the same as above
- `ttdl list --rec=any` - show all recurrent todos
- `ttdl list --rec=+` - as above

#### Filter by project, context, and tag

##### Free CLI arguments

Any free command-line argument starting with `+` is a project name, and starting with `@` is a context.
A command can contain any number of project names(the same true for contexts), in this case the filter
includes all todos that contains _any_ of listed projects names. Please note, that while passing a few
contexts means "select todos with any of provided contexts", passing a few project names and contexts
at the same time means "select todos with any of provided project names _and_ any of provided contexts".

Project names and contexts support basic matching. Append or prepend `*` to project name to match projects
which names ends or starts with the word.

- `ttdl list +proj` - show all incomplete todos that belong to project `proj`
- `ttdl list @ctxA @ctxB` - show all incomplete todos that have either context `ctxA` or context `ctxB`
- `ttdl list +projA +projB @ctx` - show all incomplete todos that belong either to project `projA` or to project `projB`, **and** have context `ctx` (`(projA ∨ projB) ∧ ctx`)
- `ttdl list +*clientA` - show all incomplete todos which project names ends with `clientA`
- `ttdl list @car*` - show all incomplete todos which have a context starting with `car`

To exclude todos containing certain project or context, prepend `-`(minus sign) before `+` or `@`:

`ttdl list +projA -@ctx` - show all incomplete todos that belongs to project `projA` and do not contains context `ctx`. (`projA ∧ ¬(ctx)`)

##### Classic CLI options

Besides using `+` and `@` to indicate project and context titles, the application supports classic CLI options to set the list of values for a filter:

- `--project` - filter by project title
- `--context` - filter by context name
- `--tag` - filter by tag name

All options accepts a comma-separated list of names (and/or patterns). If any option value matches any record value, the record is displayed.

Special values:

- `none` - select records that do not have any values (`--context none` - records without any context)
- `any` - select records that have at least one value (`--project any` - records that belong any project)

Any item can be a full name or a pattern to match as it is described in the [previous section](#free-cli-arguments).

It is possible to exclude todos that contain given projects or contexts from the output.
Minus sign `-` marks items to exclude:

`ttdl list --project projA --context "any,-ctxB"` - list all todos that belong to project `projA` and have any context except `ctxB`. (`{∀x | (x ∈ projA) ∧ ¬(x ∈ ctxB) }`)

#### Filter by date

All date-like fields(due, threshold, creation, finish) support filtering with a date range as well as a single date.
Ranges are always inclusive. Start and end of a range are separated with either `..` or `:`.
A range can be "open": in this case it is a single value with appended or prepended range separator.
For a closed range, the order of the range end is arbitrary: TTDL automatically exchanges beginning and end if needed.
Ranges support only relative dates (positive for dates in the future and negative for dates in the past) or one of special dates.

Relative dates is a list of numbers followed by suffixes (`d` - days, `w` - weeks, `m` - months, `y`- years) without separator.
E.g., `1y` in a year; `2m1w` in two months and a week; `-10d` - 10 days ago.

Special dates are:

- `none` - a date is unset; the value can be used as a single value or as only one end of a range. In latter case the filter works as two filters combined: an open range with a real date, and all todos that have the date field empty
- `today`, `tomorrow`, `yesterday` - their meaning is clear
- `first` - first day of the next month
- `-first` - first day of this month or the previous month(in case of today is the 1st)
- `last` - last day of this month or next month(in case of today is the last day)
- `-last` - last day of the previous month
- `day-of-week` - a full name of a day of week or abbreviated to 2 or 3 first letters, the nearest day-of-week in the future(it is never today)
- `-day-of-week` - a full name of a day of week or abbreviated to 2 or 3 first letters, the nearest day-of-week in the past(it is never today)

Use the following command-line options to filter by:

- due date `--due`
- threshold date `--thr`
- date of creation `--created`
- completion date `--completed`

Field `threshold` is the only field with a specific default value.
All other fields have empty default value which means the filter is off.
The default value of threshold is inclusive range `[none..today]`.
TTDL by default hides tasks that has threshold date and the threshold date is tomorrow and later.

Examples (note the difference between ranges and single values):

- `ttdl list --due today` - show all todos that are due today
- `ttdl list --due ..today` - show all todos that are due today or overdue
- `ttdl list --due today..` - show all todos that are due today or any day after today
- `ttdl list --due today..tomorrow` - show todos that are due only today or tomorrow
- `ttdl list --due tomorrow..today` - the same as above
- `ttdl list --completed -1w.. -a` - show todos that were done within the last 7 days(a week from today)
- `ttdl list --completed -first.. -a` - show todos that were done this month(or the previous month if today is the 1st)
- `ttdl list --due -first..last -a` - show todos that are due this month(including overdue ones); in corner cases(today is the 1st or the last day of month) it shows todos for 2 months range instead of a month range
- `ttdl list --created -mon..` - show todos that were created this week(or the previous week if today is Monday)
- `ttdl list --created -mon` - show todos that were created last Monday
- `ttdl list --due -2d..2d` - show todos that are either slightly overdue(1-2 days overdue) or must be done within 2 next days
- `ttdl list --due none` - show todos with undefined due date
- `ttdl list --due none..tomorrow` - show todos that are overdue, due today or tomorrow, and with empty due date
- `ttdl list --due tomorrow..none` - show todos that are due tomorrow or further in the future, and with empty due date

#### Global filter by a substring

All filters above works only for a certain todo attribute, but it is possible to filter with a simple substring or regular expression.
In this case, TTDL looks for the substring everywhere: in subject(description), contexts, and projects.
To do this, just pass a bare substring as the first argument after a command.
Note the difference between project/context filter and a simple substring one: in the latter case `*` means a character `*`, not anything before or after substring.
Use command-line option `-e` to enable fuzzy regular expression based filter.

- `ttdl list car*` - show all incomplete todos that have substring `car*` in subject, project or context
- `ttdl list car.* -e` show all incomplete todos which project, context or subject matches regular expression `car.*`

### Archive

In the long run a todo list gets full of completed tasks. They may slow down the todo list management.
If you do not need to keep completed stuff, you can delete them using command `remove`.
But if completed tasks must be kept for a while, you can archive them.
Execute `clean`(or `archive`) command and completed tasks will be moved from the actual todo list to its archive.
Cleaning up automatically removes also all empty todos.
To keep empty todos, pass the command-line option `--keep-empty`.

Archiving completed todos makes the actual todo list loading faster. Though it has a few drawbacks:

- archived todos cannot be modified (e.g, if you want to delete some archived todos, you have to do it manually in any text editor)
- there is no way to show actual and archived todos at the same time

#### How to show archived todos

To display archived todos, use option `--done`. The option enables "archive" mode: the only available command in this mode is `list` and TTDL loads `done.txt` instead of `todo.txt`. On entering this mode, the option `-A` is enabled automatically if neither `-a` nor `-A` is defined. Read the 1st paragraph of [Command line examples](#command-line-examples) to understand `-[A|a]`

### Supported commands

The list of available command is short but the commands are powerful. All commands support group operations and dry run mode, except `add` command that adds a new todo one at a time. Please, refer to section "Examples", it provides a handful of useful examples of how to filter and modify todo list.

The application determines which command to execute with the following rules:

1. If the command does not contain any free arguments, TTDL executes `list` command
2. If the first free argument is a command, TTDL executes it
3. In all other cases TTDL executes `add` command

The rules above allow a user to omit the most used commands `add` and `list`.
At the same time, the way is error-prone because every typo in command results in adding a new undesirable todo.
E.g., if you type `ttdl del 15`, TTDL, instead of removing todo at 15 position, adds a new todo with message `del 15`.
To avoid such cases and make TTDL command-line check stricter, either add `strict_mode = true` to section `global` or pass `--strict` in command-line.
It disables the rule number 3 and TTDL will always require a valid command as the first argument, if there are at least one exists.
In strict mode, `ttdl del 15` returns the error "first argument must be a command".

Commands:

- add - add a new todo;
- list - show list of todo items. By default it displays all incomplete todos;
- done - mark selected todos completed. If a todo is recurrent and contains due or threshold date(or both) the todo is marked completed and new one is created with due and threshold dates moved to the future;
- undone - remove `finished` mark from completed todos;
- remove - deletes the selected todos;
- clean - moves completed todos from main file to `done.txt`. The file `done.txt` is created(if it does not exist) in the same directory where main todo list file is located. By default, the command also removes all empty todos;
- edit - modify one or few properties for the selected todos. One exception: modifying todo's subject changes only the first selected todo, others are skipped;
- append - adds a text to the end of the selected todos (space between old text and new one is added automatically);
- prepend - inserts a new text at the beginning of the selected todos (space between old text and new one is added automatically);
- start - activate todo's timer;
- stop - stop todo's timer and update time spent on the todo;
- stats - display todo statistics: total number of todos, done and overdue ones, spent time, and detailed statistics grouped by project and context.
- postpone - push task's due date (modifies only incomplete tasks with due date defined), argument is the number of days/weeks/months/years to push the date in format: single digit and d/w/m/y without a space between them
- listprojects - show list of all project tags. Filters used by "list" are supported;
- listcontexts - show list of all context tags. Filters used by "list" are supported;

Most of the commands can be abbreviated. Please refer to built-in TTDL help to get a list of full command names and their aliases.

All commands(except `listcontexts` and `listprojects`) skip hidden tasks by default. To include hidden tasks, use `--hidden` option. See section [tags](#tags) for details.

NOTE: `done` moves a recurrent todo's due date to the next one, but it does not check if the new due date is in the future (it is by design). So, if a monthly task is 2 months overdue, you have to execute `ttdl done ID` two times to push it to the incoming month or manually set a new due date with the command `ttdl edit ID --set-due=YYYY-MM-DD`.

#### Calendar

By default, the list of todos is displayed as a table.
Command-line switch `--calendar=<range>` allows you to peek what is on your plate in a convenient way.
The displayed calendar always includes today's date.
The `range` is a single value, like `recurrence`, denotes how far in the future or or in the past (negative value) the first or the last date of the range.
The calendar displays only dates on which you have something due, i.e, the calendar filters only todos that has `due` tag.
The full `range` form is `number + range type`, e.g. `2w` - show this and the next week.
The range can be negative. In this case, TTDL displays the past weeks.
E.g, `--calendar=-2w` prints out the current and the previous week.

Note: when printing by full weeks and months, numbers `1`, `-1`, and `0` works the same and always shows only the current week or month.
So, `--calendar=1w` and `--calendar=-1w` both display the current week.

Supported range types:

- `d` - days
- `w` - weeks
- `m` - months

Also you can use some short-cuts:

- if you do not specify a number, it is defaulted to `1`. E.g, `--calendar=m` and `--calendar=1m` is the same
- if you omit range type, it defaults to `d`. E.g, `--calendar=10` means `--calendar=10d`.

In `w` and `m` modes the displayed interval always starts from the first weekday of a week or from the first date of a month,
and ends with the last day of a week or a month respectively.
E.g, if today is `2022-07-10` and you request `--calendar=1m`, TTDL outputs the calendar for the entire July: from `2022-07-01` to `2022-07-31`.
If you want to display exactly one months starting from today, mark the range strict by prepending `+`: `--calendar=+1m` for today's date `2022-07-10` prints the calendar from `2022-07-10` to `2022-08-09`.

Note: in strict mode `--calendar=1w` and `--calendar=-1w` are not equivalent.
The former displays 7 days *starting* with today.
The latter prints out 7 days *ending* with today.

Color legend:

- regular Black and White colors - "empty" day when you do not have any todo due
- Blue background - today's date
- Magenta foreground - on this day you have one todo due
- Red foreground - this day has more than one due todo

Example:

<img src="./images/todo-calendar.png" alt="Calendar example output">

In the picture:

- Today's date is 7th
- There is one todo is due on 20th
- There are more than one todo due on 6th

### Tags

The original todo.txt format describes a user-defined tags that can be used by any application for special needs. The format of a tag is `tag_name:tag_value`. The original format does not specify any tag - all are considered custom ones.

TTDL supports a few custom tags:

- `due` - a todo's due date. The tag value is in format YYYY-MM-DD;
- `t` - a todo's threshold date. The tag value is in format YYYY-MM-DD;
- `rec` - makes a todo recurrent. It makes sense only when using along with `due` tag. The tag value is the number of time intervals and one-character time interval name: `d` - every few days, `w` every few weeks, `m` - every few months, `y` - every few years. Examples: `1w` - a weekly todo, `5d` - every 5 days.
- `h` - mark a task hidden if the value of the tag is not `0`. Hidden tasks are skipped by default by all commands except autocompletion support. It allows a user to keep a hidden task with all projects and contexts for shell auto-completion without spoiling the regular tasks with unrelated contexts and projects.

You can add any number of arbitrary tags to a todo. To edit them, use `--set-tag`(add new tags or replace existing ones) or `--del-tag`(to remove tags from a todo) options.

### Hashtags

Sometimes you need to add one or few extra attributes to a todo, but you do not want to introduce any new contexts or projects.
Tags do not work in this case as they require value: you cannot create a tag with empty value.
To deal with it, use hashtags. Hashtags is a todo words that start with the symbol `#`.
Example: in a todo `buy tickets to #hockey game` there is one hashtag `#hockey`.

Use command-line options `--hashtag` to filter todo list, `--set-hashtag` to add new hashtags, `--del-hashtag` to remove hashtags, `--repl-hashtag`to replace existing hashtags.
The command `--repl-hashtag` does not append a new hashtag if the todo does not include the hashtag to be replaced.

### Time tracking

TTDL version 0.5.0 introduced time tracking feature. It consists of two new commands `start` to activate time tracking for a given todo, and `stop` to stop time tracking and update todo's time taken.

The `list` command adds an extra column `Spent` that displays total time the todo has taken by the current time.

### Statistics

Command `stats` displays general statistics followed by detailed one. If you need only general one use option `--short`.

General statistics includes the total numbers of all todos, completed, overdue, recurrent todos, and todos that missed threshold date. For the all numbers, except the total number of all todos, the percentage of all todos is displayed in parentheses. Example:

```shell
Total todos:          8
Done:                 1 (12%)
Overdue:              2 (25%)
Recurrent:            1
```

Detailed statistics groups all todos by projects and contexts and prints the subtotals for all of them. Note: because of todos can have no project or have more than one project or context, the total number of all todos is usually not equal to sum of all subgroups. Example:

```shell
Project         Context    Total      Done       Overdue    Spent
                              8(100%)    1( 12%)    2( 25%) 3.1m
-----------------------------------------------------------------
chapelshelving                2( 25%)    1( 50%)
                chapel        1( 50%)    1(100%)
                shelve        1( 50%)
-----------------------------------------------------------------
myproj                        1( 12%)               1(100%)
                bug           1(100%)               1(100%)
                ui            1(100%)               1(100%)
```

#### Notes

1. The first line with number is a grand total for the entire todo list
2. In the example above, there are total 8 todos, but only 3 of them have project tags. And `myproject` project has only one todo with 2 context tags
3. For project headers (lines which have some project and empty context) the percentage is calculated from all todos, for context totals the percentage is calculated from project total todos
4. Done and overdue percentage is always calculated from the total of the current line
5. Spent time adds low-cased letters for time spans less than a day (s - seconds, m - minutes, h - hours), and upper-cased letter for longer spans (D - days, M - months, Y - years).

### Custom formatting

The feature is introduced in version 0.8: when executing command `list` TTDL may call an external application
(a "plugin") to transform the description and/or tags. If the plugin finishes successfully
and its output is valid JSON string, `TTDL` combines values from the output and prints it instead of
default description. Only description can be changed by plugins, other columns like "priority" are fixed ones.

A plugin is a executable shell script or binary with name that follows TTDL rules. Every plugin receives
a single argument - JSON as a string, a plugin must print modified or original JSON to standard output.

If any plugin does not exist, fails or returns invalid JSON, TTDL prints the error to standard error output
and displays the unchanged original todo description. Otherwise, TTDL joins description and tags, and prints
the result.

A todo can contain any number of plugin "calls". They are executed in order of appearance in original
description. The result of a current call is passed to the next plugin only if the next plugin name is
still in the result - a plugin can remove any tags in the result to make TTDL ignore the other plugins.
So, any plugin can disable any other plugins if it removes their tags from the result.

#### How to enable custom formatting

To enable custom formatting, a todo must include at least one tag which name starts with symbol `!`.
The tag name without `!` is the plugin name(and the last part of the file name to call). By default
The full name of the file to execute is `ttdl-` + plugin name. A configuration file contains settings
in `global` section that affects file name:

- `script_ext` - value is used as a file name extension. If the value does not start with '.', the
  dot is added automatically. Default is empty value - no extension is added.
  Example: for `!tagname:value` and `script_ext=sh`, the script name is `tagname.sh`;
- `script_prefix` - its value is added before the tag name. It makes possible to, e.g., keep scripts
  in a separate directory or create a subgroup of scripts. Another usage is for Windows
  PowerShell: executing `tagname.ps1` may fail, because PowerShell wants a user to explicitly say
  that the script is in the current directory, so you have to set `script_prefix="./"` to be able to
  run PowerShell scripts from the current directory. Do not forget to add `/` at the of the prefix
  if it is a directory name - TTDL cannot decided automatically if it is a part of filename or
  a directory.

Related configuration setting defines what shell executes the script:

- `shell` - sets the shell to execute a binary/script. If not set, TTDL uses `["cmd", "/c"]` on Windows,
  and `["sh", "-cu"]` on other OS. If you want to use PowerShell on Windows, change the value to `["powershell", "-c"]`;

#### Plugin interaction

TTDL pipes a JSON with tags and optional items of a todo that is going to be displayed to a
plugin's standard input. A plugin must read stdin and after processing the JSON, the plugin
must print the result to stdout in the same JSON format. If a plugin does not need to change anything,
it must print it as is. If any plugin fails to execute or produces invalid JSON, the error is
printed to standard error and the original todo text is displayed.

The first script always gets JSON with all todo's tags and optional fields.
Three obligatory fields in the first request(a plugin may remove any of those fields):

- `description` - original todo's description that TTDL would print by default, plugins can modify it;
- `optional` - original todo's optional elements (`done`, `pri`, `created`, and `finished`);
- `specialTags` - an array of all tags and their values extracted from the todo. NOTE: if a currently
  running plugin removes a tag that belongs to a plugin that have not run yet, that plugin will be
  skipped because its tag is missing in `specialTags` at the moment when the plugin is going to run.

All the next plugins receive a JSON returned by a previous plugin. A plugin must return the modified or
original JSON by printing it to standard output. TTDL constructs and displays the modified
description returned by the very last plugin. If the final result includes any field it is displayed
as is from the JSON, otherwise the original value is printed with default formatting. That makes it
possible to, e.g., change "done" mark and replace default `x` with `√`, or you can format dates using
your native language(e.g., display `10 Sep` instead of `2020-09-10`).

A plugin may add or remove any fields in resulting JSON, that allows plugins to communicate. The only
requirement is that the result should include all fields above.

#### Notes

1. While it is OK to set any value to an existing field, the output is limited with the current
   column width (only `description` is displayed in full). E.g., if a plugin changes value of
   `created` to `12 December 2019`, in terminal TTDL prints either first 10 (default settings)
   or 8 characters(when relative dates are enabled).
2. All dates passed to the first plugin in JSON are always in format `YYYY-MM-DD` for easy
   parsing. So, even if relative dates are enabled, a plugin gets in default format.
3. All values presented in result JSON are printed as is, while all missing values are printed
   with current format settings. It results in that there is difference between: a plugin does
   not touch a standard field, and a plugin removes the standard field from the result. If a plugin
   removed non-standard field from result JSON, the field won't be printed.
   Standard fields are: "done", "pri", "created", "finished", "description", "thr", "due".

Quick example for #3. Today's date is 2020-01-18, a todo contains `2020-01-17 Test line !plug:2020 !plug2:01`,
and TTDL is launched with relative dates enabled. If plugin `plug` does not exist, it prints with default formatting:

```shell
Created  Description
1d ago   Test line !plug:2020 !plug2:01
```

If the plugin exists, it gets argument `{ "description": "Test line", "optional: [{"created": "2020-01-17"}], "specialTags: [{"!plug": "2020"}, {"plug2": "01"]}`.
Case A: the plugin returns the original JSON untouched. All values are taken from JSON:

```shell
Created  Description
2020-01- Test line !plug:2020 !plug2:01
```

Case B: the plugin removes `created` and `!plug2` from original JSON and returns `{ "description": "Test line", "optional: [], "specialTags: [{"!plug": "2020"}]}`.
Now the current formatting setting are applied to standard `created`, non-standard `!plug2` is ignored and it prints:

```shell
Created  Description
1d ago   Test line !plug:2020
```

#### Example

Let's assume there is the following line in todo.txt, and TTDL config contains `script_prefix="/home/username/ttdlscripts/"`:

```shell
2020-01-17 sprint ends !issue-cnt:project_name !issue-pct:project_name rec:2w
```

It is a recurrent todo - every 2 weeks - that notifies about the current sprint ends. We want to display
the number of opened issues for the sprint and how many percent of issues are still opened, so two tags
are added `!issue-cnt:project_name` and `!issue-pct:project_name`.

TTDL detects a tag with leading `!` and the plugin engine kicks in. The todo description is "sprint ends".
The argument for the first plugin is:

```shell
{"description": "sprint ends", \
    "optional": [ {"created": "2020-01-17" ], \
    "specialTags": [ {"!issue-cnt": "project_name"}, {"!issue-high": "project_name"} ]}
```

The first tag is `!issue-cnt`. It gives script name `/home/username/ttdlscripts/issue-cnt`. On Linux
it eventually executes:

```shell
sh -cu /home/username/ttdlscripts/issue-cnt \
    '{"description": "sprint ends", \
      "specialTags": [ {"!issue-cnt": "project_name"}, \
                       {"!issue-high": "project_name"} ]}'
```

This sprint was very good and we have no opened issues. It means that we do not need to execute the
second plugin `issue-pct` to show percentage, so the first plugin removes redundant tag from the
`specialTags` and replaces its own tag with some nice message. Also the plugin deletes `optional`
field to make it printed with default settings and original values. The plugin `issue-cnt` prints to
stdout its result:

```shell
{"description": "sprint ends - well done!", \
   "specialTags": [ {"issues:ALL-DONE": "project_name"} ]}
```

TTDL gets intermediate result, and before calling the next plugin `issue-pct`, it checks if plugin
name is still in the list. It is not found, and as it is the last plugin to call, TTDL builds
the description from the last collected output. It joins description with all tags and prints:

```shell
2020-01-17 sprint ends - well done! issues:ALL-DONE
```

### Extra features

Each command that modifies todo list supports dry run mode. The mode is enabled with an option `--dry-run`. When executing `ttdl` with the option, it finds out which todos would be changed after the command completes, then displays existing todos and their new values.

By default TTDL outputs the todo list in long mode and uses colors. To disable colors, use an option `--no-colors`. To make the output shorter, use an option `--short` to show only a few the most important fields(ID, completion mark, priority, and subject), or choose which fields to display with an option `--fields`: a comma-separated list of fields.
**NOTE**: the option `--fields` defines only a field visibility, but not its order. So, `--fields=pri,due` and `--fields=due,pri` result in the same output.

For easier reading due date, there is an option `--human` that turns dates into relative dates. So, due date 2018-11-11 can turn into `in 3d` (if the current date is 2018-11-08) or into `3d overdue`(if the current date is 2018-11-14). Using an option `--compact` makes the output even shorter: it removes all `in`s and `overdue`s. To understand whether a todo is overdue or not, just check its color: overdue ones are drawn in red color(unless you used the option `--no-colors` or modified color in TTDL config). Option `--human` supports a list of fields to show as relative ones: `ttdl l --human="due"`.

#### Syntax highlight

For better readability you can enable syntax highlighting when printing todo's subject.
The highlighting is disabled by default.
To enable it, either pass `--syntax` option  in the command line or turn it on in TTDL configuration file (see example in provided `ttdl.toml`, section `[syntax]`).
Also you can disable temporary syntax highlighting if it is enabled in the configuration by passing the command-line option `-no-syntax`.

Besides turning highlighting on and off, the configuration allows tuning the keyword colors. Default colors for keywords are:

- tag color is bright cyan
- hashtag color is cyan
- project color is bright green
- context color is green

### Human-readable dates

In addition to human-readable output, TTDL supports setting due and threshold dates in human-readable format.
Both ways work: setting them in a string with `due:` and `t:` and in subcommands `--set-due` and `set-threshold`.
Because due and threshold dates cannot be set in the past, rules for setting and displaying dates differs.
Please note, that human-readable dates are replaced with absolute dates when saving to todo file.
In the beginning you can play with human-readable dates without worrying about damaging existing todos by adding `--dry-run` to commands.
This way only displays the content but does modify anything.

The list of supported abbreviations (more can be added in the future if needed):

| Abbreviation             | Date                                                                                                                                                                 |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `today`                  | Today's date                                                                                                                                                         |
| `#`                      | `#` stands for a positive number - sets date to `#` day of the current or next month                                                                                 |
| `#-#`                    | `#-#` stands for a "month-day" - if this day of the current year is in the past it sets due date to the "next_year-month-day" and "current_year-month-day" otherwise |
| `tm`, `tmr`, `tomorrow`  | Tomorrow's date                                                                                                                                                      |
| `#d`                     | `#` is a positive number: in `#` days                                                                                                                                |
| `#w`                     | `#` is a positive number: in `#` weeks                                                                                                                               |
| `#m`                     | `#` is a positive number: in `#` months                                                                                                                              |
| `#y`                     | `#` is a positive number: in `#` years                                                                                                                               |
| `mo`, `mon`, `monday`    | nearest Monday in the future                                                                                                                                         |
| `tu`, `tue`, `tuesday`   | nearest Tuesday in the future                                                                                                                                        |
| `we`, `wed`, `wednesday` | nearest Wednesday in the future                                                                                                                                      |
| `th`, `thu`, `thursday`  | nearest Thursday in the future                                                                                                                                       |
| `fr`, `fri`, `friday`    | nearest Friday in the future                                                                                                                                         |
| `sa`, `sat`, `saturday`  | nearest Saturday in the future                                                                                                                                       |
| `su`, `sun`, `sunday`    | nearest Sunday in the future                                                                                                                                         |

1. All day of week abbreviations never set today's date. So, if the current date is Monday, `due:mon` sets the due date to the next Monday.
2. `#d`, `#w`, `#m`, and `#y` are addictive and can be grouped. Moreover, you can use the same abbreviation as many times as you want. Examples: `due:3d4d` is the same as `due:1w`; and `due:1w1d` is the same as `due:11d`
3. `#m` and `#y` does not add a constant number of days to the current date. They increase the month and year respectively with one extra rule:
   if current date is the last day of the month, the new date is the end of the month as well. E.g., `due:1m` when current date is `2020-02-29` sets due date to `2020-03-31`.
4. `#` and `#-#` never sets the date in the past. So, if `#` is less than the current day of month, the resulting date is in the next, otherwise it is in the current period(month for `#`, year for `#-#`). The same rule about the last day of month as in `3.` is applied here: `due:29` for current date `2020-02-29` sets due date to `2020-03-31`.
5. `#` and `#-#` accept values for a day in a range `[1..31]`, bigger numbers cause errors. If day number is greater than the number days in a month, the last day of the month is set. So, it is safe to use `due:31` for any month to set due date to the last day of the month. Examples: `due:31` for date `2020-02-05` sets the due date to `2020-02-29`, and `due:02-31` sets due date to `2020-02-29`.

### Custom columns

Tags are useful to keep extra information but even with syntax highlighting it may be hard to spot a certain tag inside a long subject.
Custom columns allows you to display values of selected tags in a separate columns for better readability.
In addition, it is possible to define rules to set the output color depending on a tag value.

Two steps to enable custom columns:

1. Define all custom columns in the configuration file: add an array of `[[fields]]` to the configuration. Optionally, add highlight rules for columns as an array `[[fields.rules]]`.
2. New columns are hidden by default. To show them, pass `--fields=field1,field2` in the command line.

Custom field quirks and limitations

- Every column has 5 properties: name, title, kind, width, and highlight rules. All of them, except highlight rules are mandatory. The property `width` can be set to `0` - in this case its width is chosen by TTDL
- At this moment, automatic width does not check all values of a tag to calculate its maximal width. It just sets the width depending on the column kind. So, except  columns of `date` kind, the other column widths probably would be too wide or too short most of the time
- TTDL does basic check for custom columns properties: the `color` must be valid; the `kind` must be one of `str`(its alias is `string`), `int`(its alias is `integer`), `float`, `date`, `duration`, and `bytes`; if `width` is not `0`, the length of `title` must be equal to or less than `width`
- If a value is too long, it is truncated to the column `width` - no word wrap for custom columns
- Custom columns are always displayed after the built-in ones and the order is the same in which they are defined in the configuration file
- Highlight rules provide two properties: `range` and `color`. The property `range` can be on of three types: empty string - it matches any value, list of values separated with a comma - a tag value must equal any of the list values, range - two values separated with `..`(open ranges like `..5` or `5..` are allowed)
- All rules are tested in the order of their declaration in the configuration file
- Both ends of a range are inclusive. But because of the previous bullet point, you can simulate non-inclusive ends. E.g, defining `4..` rule before `0..4` makes the latter rule non-inclusive(i.e, it becomes equal to `>=0 && < 4` because `4` always matches the former rule `>= 4`)
- For columns of `duration` kind you can set human-readable duration and TTDL compares them correctly. Available suffixes: `w` - a week, `d` - a day, `h` - an hour, `m` - a minute, `s` - a second. As a rule, `s` can be omitted - a value without any suffixes is considered as a value in seconds. A tag value can include any number of intervals in any order, e.g, `89m1d5` is the same as `1d1h29m5s` or one day and one hour and 29 minutes and 5 seconds
- For columns of `bytes` kind you can standard suffixes. One letter ones: `k`, `m`, `g`, `t`, `p`, `e`. Two letter ones: `kb`, `mb`, `gb`, `tb`, `pb` , `eb`. Three letter ones: `kib`, `mib`, `gib`, `tib`, `pib`, `eib`. Note: at this moment all suffixes are multiple of `1024`, so `k` = `kb` = `kib` = `1024 bytes`. A value without suffixes is in bytes. In opposite to `duration`, this kind of values can have only one suffix. So, e.g., `1mb2kb` is invalid value and it is treated as an empty value
- Rules for `date` columns can include special dates: `yesterday`, `today`, `tomorrow`, `soon`(currently it is today + 7 days), `last` - the last day of the month, `first` - the first day of the month, day of week(`friday` - this upcoming Friday). It makes rules more flexible
- While rules for `date` column support special dates, the tag value must be a date in format `YYYY-MM-DD`.

#### Custom column example

Let's assume, you manage a team that includes you(`me`), technical writer(`Tina`), two junior developers(`Mike` and `Andy`), and a few middle developers.
A task can have an estimation time and contain a day when you should check how the task is going (it is not the same as due date).

For person responsible for a task you choose a tag `who:`(kind `string`), for estimation time - a tag `est:`(kind `duration`), and for check date - a tag `chk:`(kind `date`).

Now let's add colors to quickly read todos. You want to mark your tasks with red color, juniors' with bright blue color, documentation tasks are green color, the rest tasks has default color.
Let's also separate tasks by estimated time:

- green color for tiny tasks (less than or equal to 1 hour)
- blue color for small tasks (between 1 hour and 4 hours)
- red color for huge tasks (1 week or longer)

The last highlight is for check date. Overdue checks are red.
Checks that should be done first(i.e, they are coming today or `soon`) are bright yellow.

Now open `ttdl.toml` and add the custom columns with rules to it:

```toml
[[fields]]
name = "who" # tag name
title = "Who" # Title of the column in the output
kind = "string" # type of the tag value. If you are not going to use `fields.rules`
                # for coloring, it is safe to set `string` kind for every column.
width = 8 # column width
[[fields.rules]]
range = "Tina"
color = "green"
[[fields.rules]]
range = "Andy,Mike" # either of juniors: Andy or Mike
color = "bright blue"
[[fields.rules]]
range = "me" # my tasks
color = "red"

[[fields]]
name = "est"
title = "Estimate"
kind = "duration"
width = 8
[[fields.rules]]
range = "..1h"  # shorter than or equal to 1 hour
color = "green"
[[fields.rules]]
range = "1h..4h" # >= 1 hour and <= 4 hours
color = "blue"
[[fields.rules]]
range = "1w.." # 1 week or longer
color = "red"

[[fields]]
name = "chk"
title = "Check"
width = 0
kind = "date"
[[fields.rules]]
range = "..yesterday" # overdue check
color = "red"
[[fields.rules]]
range = "today..soon" # between today and today+7 days, inclusive
color = "bright yellow"
```

Let's assume that today is `2022-12-31` and your `todo.txt` looks like this:

```
2022-10-10 update docs est:1w who:Tina chk:2023-01-09
2022-07-20 design mail server who:me est:1d4h
2022-07-25 fix crash in logger est:2h who:John
2022-10-10 upgrade dependencies who:Mike est:4h chk:2022-12-27
2022-10-11 write test for new API call who:Andy est:1h chk:2023-01-03
```

After that, if your the command `ttdl --fields=who,est,chk`, you'll see this output in the terminal window:

<img src="./images/custom-columns.png" alt="Custom columns">

## Command line examples

By default todos from a given range are processed only if they are incomplete. To process all(both done and incomplete), add an option `--all` or `-a`. To process only done todos, add an option `-A`. NOTE: the only exception is the command `clean`|`archive`, it enables option `-A` automatically if `--all` is not present in command line.

### List and filter

| Command                    | Description                                                                                                                      |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `ttdl l 2`                 | show a single todo with ID 2                                                                                                     |
| `ttdl l 2-5`               | show todos with ID from 2 through 5                                                                                              |
| `ttdl l 2,5`               | show only todos with ID 2 and 5                                                                                                  |
| `ttdl l -s=proj,pri`       | show all todos sorted by their project and by priority inside each project                                                       |
| `ttdl l "car*"`            | list todos which have substring `car*` in their subject, project or context                                                      |
| `ttdl l "car*" -e`         | list todos which have subject, project or context matched regular expression `car*`                                              |
| `ttdl l "car"`             | list todos which have substring `car` in their subject, project or context                                                       |
| `ttdl l --pri=a`           | show todos with the highest priority A                                                                                           |
| `ttdl l --pri=b+`          | show todos with priority B and higher (only A and B in this case)                                                                |
| `ttdl l +car +train`       | show todos which related either to `car` or to `train` projects                                                                  |
| `ttdl l +my* @*tax`        | show todos that have a project tag starts with `my` and a context ends with `tax`                                                |
| `ttdl l --due=tomorrow`    | show todos that are due tomorrow                                                                                                 |
| `ttdl l --due=soon`        | show todos which are due in less than a few days, including overdue ones (the range is configurable and default value is 7 days) |
| `ttdl l --due=overdue`     | show overdue todos                                                                                                               |
| `ttdl l --due=none`        | show todos that does not have due date                                                                                           |
| `ttdl l --due=today`       | show todos that are due today                                                                                                    |
| `ttdl l +myproj @ui @rest` | show todos related to project 'myproj' which contains either 'ui' or 'rest' context                                              |
### Add a new todo

| Command                                                                       | Description                                                                     |
| ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `ttdl a "send tax declaration +personal @finance @tax due:2018-04-01 rec:1y"` | add a new recurrent todo(yearly todo) with a due date first of April every year |

### Done (undone)

| Command      | Description                               |
| ------------ | ----------------------------------------- |
| `ttdl d 2-5` | mark todos with IDs from 2 through 5 done |

### Clean up the list

| Command                   | Description                                                                                         |
| ------------------------- | --------------------------------------------------------------------------------------------------- |
| `ttdl rm 2-5`             | delete incomplete todos with IDs from 2 through 5                                                   |
| `ttdl rm 2-5 -a`          | delete both done and incomplete todos with IDs from 2 through 5                                     |
| `ttdl rm 2-5 -A`          | delete all done todos with IDs from 2 through 5                                                     |
| `ttdl clean 2-5 --wipe`   | delete all completed todos with IDs from 2 through 5. It does the same as the previous command does |
| `ttdl clean 2-5`          | move all completed todos with IDs from 2 through 5 to done.txt                                      |
| `ttdl clean --keep-empty` | move all completed todos to done.txt and erase empty todos from todo.txt                            |
| `ttdl clean --keep-empty` | move all completed todos to done.txt and keep empty todos in todo.txt                               |

### Modify todo list

| Command                                      | Description                                                                                                                                |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `ttdl e 2-5 "new subject"`                   | only the first incomplete todo with ID between 2 and 5 changes its subject (in this case todo with ID equals 2 gets subject "new subject") |
| `ttdl e +proj --repl-ctx=bug1010@bug1020`    | replace context `bug1010` with `bug1020` for all incomplete todos that related to project `proj`                                           |
| `ttdl e @customer_acme --set-due=2018-12-31` | set due date 2018-12-31 for all incomplete todos that has `customer_acme` context                                                          |
| `ttdl e @customer_acme --set-due=none`       | remove due date 2018-12-31 for all incomplete todos that has `customer_acme` context                                                       |
| `ttdl e --pri=none --set-pri=z`              | set the lowest priority for all incomplete todos which do not have a priority set                                                          |
| `ttdl e @bug1000 --set-pri=+`                | increase priority for all incomplete todos which have context `bug1000`, todos which did not have priority set get the lowest priority `z` |
| `ttdl postpone 3 5d`                         | push back due date of task #3 by 5 days                                                                                                    |

### Use human-readable dates

All examples are for the current date `2020-07-11`

| Command                            | Description                                                                                  |
| ---------------------------------- | -------------------------------------------------------------------------------------------- |
| `ttdl "fix by monday due:mon"`     | Adds a new todo with a content `fix by monday due:2020-07-13`                                |
| `ttdl "update docs t:1m due:1m2w"` | Adds a new todo with a content `update docs t:2020-08-11 due:2020-08-25`                     |
| `ttdl e 3 --set-due 1m`            | Updates the third todo with new due date `2020-08-11`                                        |
| `ttdl e 3 --set-due 1m`            | For the current date `2020-02-29` it sets due date to the end of the next month `2020-03-31` |

### Show all project or context tags

| Command                | Description                               |
| ---------------------- | ----------------------------------------- |
| `ttdl lc`              | show all context tags                     |
| `ttdl listcon @phone*` | show all context tags starting with phone |
| `ttdl lp`              | show all project tags                     |
| `ttdl listproj +*car*` | show all project tags containing _car_    |

These commands could be used to implement tag completion in your editor / shell.
