## Vorgangsliste

title = Vorgänge
    .description = Alles, woran das Team arbeitet, über alle Projekte hinweg.
new_issue = Neuer Vorgang
    .description = Beschreibe die Arbeit so, dass jemand anderes sie übernehmen kann.

search = Vorgänge durchsuchen
    .placeholder = Titel oder Schlüssel wie WEB-12 suchen
project_filter = Projekt
    .all = Alle Projekte
status_filter = Status
    .all = Alle Status
priority_filter = Priorität
    .all = Alle Prioritäten
sort = Sortieren nach

sort_updated = Zuletzt aktualisiert
sort_newest = Neueste zuerst
sort_oldest = Älteste zuerst
sort_priority = Höchste Priorität

no_results = Keine Vorgänge gefunden
    .description = Kein Vorgang passt zu deiner Suche und deinen Filtern.
clear_filters = Filter zurücksetzen

column_key = Schlüssel
column_title = Titel
column_status = Status
column_priority = Priorität
column_assignee = Zuständig
column_updated = Aktualisiert
column_due = Fällig

status_backlog = Backlog
status_todo = Zu erledigen
status_in_progress = In Arbeit
status_in_review = In Prüfung
status_done = Erledigt

priority_urgent = Dringend
priority_high = Hoch
priority_medium = Mittel
priority_low = Niedrig

kind_task = Aufgabe
kind_bug = Fehler
kind_feature = Funktion

unassigned = Nicht zugewiesen
# Wird von Screenreadern nach einem überschrittenen Fälligkeitsdatum vorgelesen.
overdue = (überfällig)

## Vorgangsformular

field_project = Projekt
    .choose = Projekt auswählen
field_title = Titel
field_description = Beschreibung
    .placeholder = Was soll passieren, und warum?
field_kind = Typ
field_status = Status
field_priority = Priorität
field_assignee = Zuständig
field_due = Fällig am

create_issue = Vorgang erstellen
save = Änderungen speichern
cancel = Abbrechen

error_project_missing = Wähle ein Projekt aus.
error_title_missing = Gib einen Titel ein.
error_title_too_long = Verwende höchstens { $max } Zeichen.
error_description_too_long = Verwende höchstens { $max } Zeichen.
error_assignee_unknown = Wähle ein Teammitglied aus.
error_due_invalid = Gib ein Datum ein.

## Vorgangsdetails

edit = Bearbeiten
# `$key` ist der Vorgangsschlüssel, etwa WEB-12.
edit_issue = { $key } bearbeiten
delete = Löschen

description = Beschreibung
no_description = Noch keine Beschreibung.
details = Details
reporter = Erstellt von
no_due_date = Keins
created = Erstellt
updated = Aktualisiert

comments = Kommentare
comment_count =
    { $count ->
        [0] Noch keine Kommentare
        [one] { $count } Kommentar
       *[other] { $count } Kommentare
    }
add_comment = Kommentar hinzufügen
    .placeholder = Schreibe einen Kommentar
error_comment_missing = Schreibe zuerst einen Kommentar.
error_comment_too_long = Verwende höchstens { $max } Zeichen.

# `$key` ist der Vorgangsschlüssel, etwa WEB-12.
confirm_delete = { $key } löschen?
    .description = Der Vorgang und seine Kommentare werden für alle entfernt. Das lässt sich nicht rückgängig machen.
keep_issue = Vorgang behalten
delete_issue = Vorgang löschen
