# A date and time of day in UTC.
timestamp = { DATETIME($at, dateStyle: "medium", timeStyle: "short") } UTC

just_now = just now
minutes_ago =
    { $count ->
        [one] { $count } minute ago
       *[other] { $count } minutes ago
    }
hours_ago =
    { $count ->
        [one] { $count } hour ago
       *[other] { $count } hours ago
    }
days_ago =
    { $count ->
        [one] { $count } day ago
       *[other] { $count } days ago
    }

role_admin = Admin
role_member = Member
role_viewer = Viewer
