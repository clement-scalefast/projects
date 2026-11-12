welcome = Willkommen zurück, { $name }
    .description = So steht es um die Arbeit des Teams.

open_issues = Offene Vorgänge
    .note = { $count } in Arbeit
created = Diese Woche erstellt
completed = Diese Woche erledigt
# Vergleicht einen Wert der letzten sieben Tage mit den sieben Tagen davor.
week_before = { $count } in der Woche davor
overdue = Überfällig
    .note = Offene Vorgänge nach ihrem Fälligkeitsdatum

my_issues = Mir zugewiesen
    .description =
        { $count ->
            [0] Nichts wartet auf dich.
            [one] { $count } offener Vorgang, der dringendste zuerst.
           *[other] { $count } offene Vorgänge, die dringendsten zuerst.
        }
no_issues = Du hast alles erledigt.
view_all = Alle { $count } ansehen

projects = Projekte
    .description = Erledigte Vorgänge von allen Vorgängen.
