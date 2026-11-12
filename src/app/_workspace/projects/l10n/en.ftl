## Project list

title = Projects
    .description = Each project has its own board and issue keys.
new_project = New project
    .description = A project groups related issues under one key.
no_projects = There are no projects yet.

# How many of a project's issues are done.
done_of = { $done } of { $total } done
open_count = { $count } open

## Project form

field_name = Name
# The short prefix of the project's issue keys.
field_code = Key
field_description = Description
field_lead = Lead
    .choose = Choose a lead

create_project = Create project
save = Save changes
cancel = Cancel

error_name_missing = Enter a name.
error_name_too_long = Use { $max } characters or fewer.
error_code_missing = Enter a key.
error_code_invalid = Use { $min } to { $max } letters.
error_code_taken = Another project uses this key.
error_description_too_long = Use { $max } characters or fewer.
error_lead_missing = Choose a team member.

## Board

settings = Settings
    .description = Changing the key changes the key of every issue in the project.
new_issue = New issue
empty_column = No issues
# The label of a button that moves a card. `$status` is a column name, such as "Done".
move_to = Move to { $status }
