# Ein Datum mit Uhrzeit in UTC.
timestamp = { DATETIME($at, dateStyle: "medium", timeStyle: "short") } UTC

just_now = gerade eben
minutes_ago =
    { $count ->
        [one] vor { $count } Minute
       *[other] vor { $count } Minuten
    }
hours_ago =
    { $count ->
        [one] vor { $count } Stunde
       *[other] vor { $count } Stunden
    }
days_ago =
    { $count ->
        [one] vor { $count } Tag
       *[other] vor { $count } Tagen
    }

role_admin = Admin
role_member = Mitglied
role_viewer = Betrachter
