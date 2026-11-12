## Issue list

title = Issues
    .description = Everything the team is working on, across all projects.
new_issue = New issue
    .description = Describe the work so someone else can pick it up.

search = Search issues
    .placeholder = Search titles or keys like WEB-12
project_filter = Project
    .all = All projects
status_filter = Status
    .all = All statuses
priority_filter = Priority
    .all = All priorities
sort = Sort by

sort_updated = Recently updated
sort_newest = Newest first
sort_oldest = Oldest first
sort_priority = Highest priority

no_results = No issues found
    .description = No issues match your search and filters.
clear_filters = Clear filters

column_key = Key
column_title = Title
column_status = Status
column_priority = Priority
column_assignee = Assignee
column_updated = Updated
column_due = Due

status_backlog = Backlog
status_todo = To do
status_in_progress = In progress
status_in_review = In review
status_done = Done

priority_urgent = Urgent
priority_high = High
priority_medium = Medium
priority_low = Low

kind_task = Task
kind_bug = Bug
kind_feature = Feature

unassigned = Unassigned
# Read by screen readers after a due date that has passed.
overdue = (overdue)

## Issue form

field_project = Project
    .choose = Choose a project
field_title = Title
field_description = Description
    .placeholder = What needs to happen, and why?
field_kind = Type
field_status = Status
field_priority = Priority
field_assignee = Assignee
field_due = Due date

create_issue = Create issue
save = Save changes
cancel = Cancel

error_project_missing = Choose a project.
error_title_missing = Enter a title.
error_title_too_long = Use { $max } characters or fewer.
error_description_too_long = Use { $max } characters or fewer.
error_assignee_unknown = Choose a team member.
error_due_invalid = Enter a date.

## Issue details

edit = Edit
# `$key` is the issue key, such as WEB-12.
edit_issue = Edit { $key }
delete = Delete

description = Description
no_description = No description yet.
details = Details
reporter = Reporter
no_due_date = None
created = Created
updated = Updated

comments = Comments
comment_count =
    { $count ->
        [0] No comments yet
        [one] { $count } comment
       *[other] { $count } comments
    }
add_comment = Add comment
    .placeholder = Leave a comment
error_comment_missing = Write a comment first.
error_comment_too_long = Use { $max } characters or fewer.

# `$key` is the issue key, such as WEB-12.
confirm_delete = Delete { $key }?
    .description = The issue and its comments are removed for everyone. This cannot be undone.
keep_issue = Keep issue
delete_issue = Delete issue
