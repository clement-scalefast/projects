welcome = Welcome back, { $name }
    .description = Here is where the team's work stands.

open_issues = Open issues
    .note = { $count } in progress
created = Created this week
completed = Completed this week
# Compares a figure for the last seven days with the seven days before.
week_before = { $count } the week before
overdue = Overdue
    .note = Open issues past their due date

my_issues = Assigned to me
    .description =
        { $count ->
            [0] Nothing is waiting for you.
            [one] { $count } open issue, most urgent first.
           *[other] { $count } open issues, most urgent first.
        }
no_issues = You are all caught up.
view_all = View all { $count }

projects = Projects
    .description = Done issues out of all issues.
