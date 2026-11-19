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

# Ordnereigenschaften (Inhalt & Speicherplatz)
prop-contents-label = Inhalt
prop-free-space-label = Freier Speicher
hidden-items-count = { $count ->
    [one] 1 versteckt
   *[other] { $count } versteckt
}

# Tooltips
tip-new-tab = Neuen Tab öffnen (Ctrl+T)
tip-close-tab = Tab schließen (Ctrl+W)
tip-language = Oberflächensprache ändern
tip-theme = Zwischen hellem und dunklem Design wechseln
tip-back = Zurück (Alt+←)
tip-forward = Vorwärts (Alt+→)
tip-up = Übergeordneter Ordner (Alt+↑)
tip-home = Zum Benutzerordner wechseln
tip-edit-path = Pfad direkt bearbeiten (Ctrl+L)
tip-deep-search = Rekursiv in Unterordnern suchen (Ctrl+Shift+F)
tip-view-grid = Rasteransicht
tip-view-list = Detailtabellenansicht
tip-view-columns = Spaltenansicht (Miller)
tip-new-item = Neuen Ordner oder neue Datei erstellen
tip-group-folders = Ordner im Raster vor Dateien gruppieren
tip-zoom-out = Kachelgröße verkleinern (Ctrl+-)
tip-zoom-reset = Standardgröße wiederherstellen (Ctrl+0)
tip-zoom-in = Kachelgröße vergrößern (Ctrl++)
tip-sort = Sortierkriterium wechseln (Name, Größe, Typ, Geändert)
tip-sort-dir = Auf- oder absteigende Sortierung wechseln
tip-filter = Elemente nach Ordner oder Erweiterung filtern
tip-hidden = Versteckte Dateien ein- oder ausblenden (Ctrl+H)
tip-terminal = Terminal in diesem Ordner öffnen (Ctrl+Shift+T)
tip-favorite = Ordner an Favoriten anheften oder lösen
tip-inspector = Inspektor-Bereich ein- oder ausblenden (Alt+I)
tip-preferences = Einstellungen öffnen (Rechtsklick für Schnellmenü)
tip-toggle-sidebar = Seitenleiste ein- oder ausklappen (Ctrl+B)

# Benutzerorte & Laufwerke ein-/aushängen
music = Musik
videos = Videos
public-dir = Öffentlich
templates = Vorlagen
filesystem-root = Dateisystem
mount-drive = Einhängen
unmount-drive = Aushängen
unmounted-label = Ausgehängt
refresh-drives = Laufwerke aktualisieren
tip-expand-home = Ordner im Benutzerverzeichnis ein- oder ausklappen
open-as-root = Als Root öffnen
root-mode-badge = Root-Modus (Erhöhte Rechte)
grid-center-label = Elementausrichtung im Raster
grid-align-left = Links nach rechts
grid-align-center = Zentriert
tip-grid-alignment = Rasterausrichtung umschalten (Links nach rechts oder Zentriert)
tip-system-icons = Zwischen Standard- und Systemsymbolen umschalten
perm-allow-exec = Ausführung als Programm freigeben
open-with = Öffnen mit...
prop-tab-open-with = Öffnen mit
open-with-prompt = Anwendung auswählen, um diese Datei und andere desselben Typs zu öffnen:
open-with-default-app = Standardanwendung
open-with-recommended-apps = Empfohlene Anwendungen
open-with-other-apps = Weitere Anwendungen
open-with-custom-hint = Oder einen benutzerdefinierten Befehl bzw. eine ausführbare Datei angeben:
open-with-custom-placeholder = z. B. mpv --no-terminal oder /pfad/programm
open-with-add-to-list = Zur Liste hinzufügen
open-with-set-default = Als Standard festlegen
open-with-reset-default = Auf Systemstandard zurücksetzen
open-with-open-once = Jetzt öffnen
open-with-loading = Installierte Anwendungen werden gesucht...
open-with-empty = Keine passenden Anwendungen gefunden.
open-with-saved = Standardanwendung aktualisiert.
open-with-reset-done = Systemstandard wiederhergestellt.
open-with-pick-exec = Ausführbare Datei auswählen

# Stapelumbenennung
br-title = Mehrfach umbenennen
br-menu-item = Mehrfach umbenennen...
br-clear-rules = Regeln zurücksetzen
br-rules-header = Was möchten Sie tun?
br-preview-header = Live-Vorschau
br-col-status = Status
br-col-original = Aktueller Name
br-col-new = Neuer Name
br-unchanged = (unverändert)
br-skipped-folder = (Ordner übersprungen)
br-conflict-internal = Konflikt: Zielname doppelt
br-conflict-external = Konflikt: existiert bereits
br-invalid-name = Ungültiger Dateiname
br-replace-title = Suchen & Ersetzen
br-find-label = Suchen
br-replace-label = Ersetzen durch
br-where-label = Bereich
br-scope-anywhere = Überall
br-scope-start-name = Nur am Namensanfang
br-scope-end-name = Nur am Namensende
br-scope-word-start = Wortanfang
br-scope-word-end = Wortende
br-scope-whole-word = Ganzes Wort
br-match-case = Groß-/Kleinschreibung
br-first-only = Nur erstes Vorkommen
br-advanced-regex = Erweiterte Optionen (Regex)
br-use-regex = Regulären Ausdruck verwenden (Regex)
br-remove-title = Text / Zeichen entfernen
br-remove-first = Erste Zeichen
br-remove-last = Letzte Zeichen
br-remove-text-label = Text entfernen
br-remove-digits = Ziffern (0-9)
br-remove-symbols = Symbole
br-collapse-spaces = Doppelte Leerzeichen
br-insert-title = Text einfügen
br-insert-text-label = Einzufügender Text
br-insert-pos-label = Position
br-insert-start = Am Namensanfang
br-insert-end = Am Namensende
br-insert-offset-start = An Position (ab Anfang)
br-insert-offset-end = An Position (ab Ende)
br-case-title = Groß- / Kleinschreibung
br-case-keep = Keine Änderung
br-case-upper = ALLES GROSSBUCHSTABEN
br-case-lower = alles kleinbuchstaben
br-case-title-words = Jedes Wort Groß
br-case-sentence = Nur ersten Buchstaben groß
br-number-title = Fortlaufende Nummerierung
br-number-append = Am Namensende
br-number-prepend = Am Namensanfang
br-number-replace = Ganzen Namen ersetzen
br-number-start = Start
br-number-step = Schritt
br-number-digits = Stellen
br-number-sep = Trennzeichen
br-sort-label = Reihenfolge
br-sort-selection = Auswahlreihenfolge
br-sort-name-asc = Name (A-Z)
br-sort-name-desc = Name (Z-A)
br-sort-date-desc = Neueste zuerst
br-sort-date-asc = Älteste zuerst
br-sort-size-asc = Kleinste zuerst
br-sort-size-desc = Größte zuerst
br-media-title = Foto- / Musikdaten (EXIF & ID3)
br-media-exif-ymd = Aufnahmedatum (JJJJ-MM-TT)
br-media-exif-datetime = Datum & Uhrzeit
br-media-exif-camera = Kameramodell
br-media-id3-track = Titelnummer
br-media-id3-artist = Interpret
br-media-id3-album = Album
br-media-id3-title = Titel
br-media-id3-year = Jahr
br-media-loading = Lese EXIF/ID3-Tags...
br-ext-title = Dateiendung ändern
br-ext-keep = Original behalten
br-ext-lower = kleinschreibung (.jpg)
br-ext-upper = GROSSSCHREIBUNG (.JPG)
br-ext-replace = Ändern in
br-ext-remove = Endung entfernen
br-include-folders = Ordner einbeziehen
br-apply = Anwenden
br-applying = Benenne um...
br-undo-last = Letzte Umbenennung rückgängig
br-readonly-error = Stapelumbenennung wird im Papierkorb oder in Archiven nicht unterstützt.
