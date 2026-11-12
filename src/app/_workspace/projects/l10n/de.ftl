## Projektliste

title = Projekte
    .description = Jedes Projekt hat ein eigenes Board und eigene Vorgangsschlüssel.
new_project = Neues Projekt
    .description = Ein Projekt fasst zusammengehörige Vorgänge unter einem Schlüssel zusammen.
no_projects = Es gibt noch keine Projekte.

# Wie viele Vorgänge eines Projekts erledigt sind.
done_of = { $done } von { $total } erledigt
open_count = { $count } offen

## Projektformular

field_name = Name
# Das kurze Präfix der Vorgangsschlüssel des Projekts.
field_code = Schlüssel
field_description = Beschreibung
field_lead = Leitung
    .choose = Leitung auswählen

create_project = Projekt erstellen
save = Änderungen speichern
cancel = Abbrechen

error_name_missing = Gib einen Namen ein.
error_name_too_long = Verwende höchstens { $max } Zeichen.
error_code_missing = Gib einen Schlüssel ein.
error_code_invalid = Verwende { $min } bis { $max } Buchstaben.
error_code_taken = Ein anderes Projekt verwendet diesen Schlüssel.
error_description_too_long = Verwende höchstens { $max } Zeichen.
error_lead_missing = Wähle ein Teammitglied aus.

## Board

settings = Einstellungen
    .description = Ein neuer Schlüssel ändert den Schlüssel aller Vorgänge des Projekts.
new_issue = Neuer Vorgang
empty_column = Keine Vorgänge
# Die Beschriftung einer Schaltfläche, die eine Karte verschiebt. `$status` ist ein
# Spaltenname, etwa „Erledigt“.
move_to = Nach „{ $status }“ verschieben
