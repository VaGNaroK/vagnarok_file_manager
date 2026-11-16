# Deutsch (DE) — Locale für Vagnarok File Manager

# Seitenleiste & Navigation
quick-access = SCHNELLZUGRIFF
drives = LAUFWERKE
tags-and-system = TAGS & SYSTEM
documents = Dokumente
home = Persönlicher Ordner
desktop = Schreibtisch
downloads = Downloads
pictures = Bilder
code-dev = Code / Dev
starred = Favoriten
trash = Papierkorb
preferences = Einstellungen

# Kopfzeile & Suche
search-placeholder = In diesem Ordner suchen...
deep-search = Unterordner
deep-search-title = Tiefensuche (Fuzzy)
deep-search-empty = Keine passenden Dateien in Unterordnern gefunden.
searching = Durchsuche Unterordner...
no-active-tab = Kein aktiver Tab
follow-system = System (Auto)

# Aktionsleiste, Ansicht & Canvas
new-item = + Neu
sort = Sortieren
hidden = Versteckte
folders = ORDNER
files = DATEIEN
total-size = DIREKTE GRÖSSE
open-terminal = Terminal
pin-favorite = Anheften
unpin-favorite = Lösen
remove-favorite = Aus Favoriten entfernen
filter-all = Alle
filter-ext = Filter
directories-only = Ordner
archive-readonly = Komprimiertes Archiv (schreibgeschützt)
grid-group-label = Ordner im Raster gruppieren
dirs-first-label = Ordner beim Sortieren zuerst
recursive-size-label = GESAMTGRÖSSE
empty-trash = Papierkorb leeren
restore-item = Wiederherstellen
confirm-empty-trash-msg = Alle Elemente im Systempapierkorb endgültig löschen?
trash-emptied = Papierkorb geleert.
trash-restored = Element(e) am ursprünglichen Ort wiederhergestellt.
empty-subdir = Leerer Unterordner
loading = Ordner wird geladen...
empty-dir = Leerer Ordner
go-back = Zurück
retry = Erneut versuchen
root-fs = System
dir-badge = ORD
file-badge = DAT

# Spalten der Detailtabelle
col-name = Name
col-size = Größe
col-type = Typ
col-perms = Berechtigungen
col-modified = Geändert

# Inspektor & QuickLook
inspector-title = Inspektor
inspector-empty = Element auswählen, um Metadaten, Rechte und Prüfsumme anzuzeigen.
permissions = Berechtigungen
modified = Geändert
created = Erstellt
checksum = SHA-256
mime-type = MIME-Typ
computing-hash = wird berechnet...
quicklook-hint = Leertaste: QuickLook
quicklook-title = QuickLook
close = Schließen
binary-preview-hint = Binär- oder zu große Datei — keine direkte Vorschau verfügbar.
git-status-label = Git-Status

# Fußzeile & Übertragung
selected-none = Ausgewählt: Keine
free-of = frei von
clipboard-copy-tag = Kopiert
clipboard-cut-tag = Ausgeschnitten
transferring = Übertrage...

# Dateioperationen & Dialoge
new-folder = Neuer Ordner
new-file = Neue Datei
rename = Umbenennen (F2)
copy = Kopieren (Ctrl+C)
cut = Ausschneiden (Ctrl+X)
paste = Einfügen (Ctrl+V)
move-to-trash = In Papierkorb (Entf)
delete-permanent = Endgültig löschen (Shift+Entf)
confirm = Bestätigen
cancel = Abbrechen
create = Erstellen
name-label = Name
invalid-name = Ungültiger Name.
confirm-trash-msg = Ausgewählte Elemente in den Papierkorb verschieben?
confirm-delete-msg = Dies löscht dauerhaft von der Festplatte und kann nicht rückgängig gemacht werden.
conflict-title = Zielkonflikt
conflict-msg = Einige Elemente existieren bereits am Ziel. Wie möchten Sie fortfahren?
conflict-rename = Beide behalten (Umbenennen)
conflict-overwrite = Überschreiben
conflict-skip = Vorhandene überspringen
open-item = Öffnen
open-in-new-tab = In neuem Tab öffnen
copy-path = Pfad kopieren
select-all = Alles auswählen (Ctrl+A)
path-copied = Pfad kopiert
properties = Eigenschaften

# Einstellungen
prefs-title = Einstellungen
theme-label = Design
lang-label = Sprache
hero-card-label = Ordner-Übersichtskarte anzeigen
decorations-label = Fenstertitelleiste
default-view-label = Standardansicht
view-grid = Raster
view-list = Liste
view-columns = Spalten
window-bar-native = Natives WM
window-bar-csd = Integriertes CSD

# Eigenschaften-Fenster
location-label = Speicherort
copy-sha256 = SHA-256 kopieren
hash-copied = SHA-256 kopiert
properties-header-prefix = { "Eigenschaften: " }
prop-tab-general = Allgemein
prop-tab-security = Sicherheit
prop-tab-history = Verlauf
prop-tab-metadata = Metadaten
prop-tab-permissions = Berechtigungen
prop-kind-label = Typ
prop-system-mime-label = System / MIME
prop-accessed-label = Letzter Zugriff
prop-quick-props = Schnelleigenschaften
prop-readonly = Schreibgeschützt
prop-hidden = Versteckt
prop-advanced-settings = Erweiterte Einstellungen
prop-confirm = Bestätigen
prop-apply = Anwenden
prop-open-default = Standard öffnen
prop-owner = Eigentümer
prop-group = Gruppe
prop-others = Andere
prop-octal-mode = Oktalmodus
prop-symlink-target = Symlink-Ziel
prop-archive-notice = Schreibgeschützt (im komprimierten Archiv)
prop-applied-notice = Eigenschaften übernommen.
prop-pending-changes = Ausstehende Änderungen
prop-calculating = Wird berechnet...
prop-sha-na-folders = Für Ordner nicht verfügbar
prop-sha-computing = SHA-256-Hash wird berechnet...
yes-label = Ja
no-label = Nein
ext-label = Erweiterung
perm-read = Lesen
perm-write = Schreiben
perm-execute = Ausführen

# Elementtypen
kind-folder = Dateiordner
kind-symlink-dir = Symbolischer Link (Ordner)
kind-symlink-file = Symbolischer Link (Datei)
kind-special = Spezialeintrag
kind-file = Datei
kind-file-ext = Datei (.{ $ext })

# Pluralisierung & Formatierung
items-count = { $count ->
    [one] 1 Element
   *[other] { $count } Elemente
}
folders-count = { $count ->
    [one] 1 Ordner
   *[other] { $count } Ordner
}
files-count = { $count ->
    [one] 1 Datei
   *[other] { $count } Dateien
}
selected-multi = { $count ->
    [one] 1 Element ({ $size })
   *[other] { $count } Elemente ({ $size })
}
exact-bytes = ({ $bytes } Bytes)
age-now = jetzt
age-minutes = vor { $m } Min.
age-hours = vor { $h } Std.
age-days = vor { $d } Tg.
ts-today = Heute, { $time }
ts-yesterday = Gestern, { $time }

# Einstellungen-Tabs & Über
prefs-tab-general = Allgemein
about-tab = Über
about-subtitle = Nativer Hochleistungs-Dateimanager für Linux & Windows
about-version-label = Version
about-developer-label = Entwickler
about-developer-value = in Entwicklung
about-stage-label = Phase
about-stage-alpha = Alpha — Entwicklungsvorschau
about-license-label = Lizenz
about-stack-label = Engine / Stack
about-copy-info = Systeminfos kopieren
about-info-copied = Systeminfos kopiert
