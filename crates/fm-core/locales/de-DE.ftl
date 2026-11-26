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

# Entpacken und Komprimieren
arch-extract-here = Hier entpacken
arch-extract-subdir = In Unterordner entpacken
arch-compress-as = Komprimieren als...
arch-add-7z = Zu .7z-Archiv hinzufügen
arch-modal-title = Komprimieren als...
arch-output-name = Archivname
arch-destination = Zielordner
arch-format-label = Archivformat
arch-level-label = Kompressionsstufe
arch-format-7z = .7z (7-Zip — Hohe Kompression)
arch-format-zip = .zip (ZIP — Universell)
arch-format-tar = .tar (TAR — Unkomprimiert)
arch-format-targz = .tar.gz (TAR.GZ — Gzip-komprimiert)
arch-level-store = Speichern (ohne Kompression)
arch-level-min = Minimum (schnell)
arch-level-normal = Normal (ausgewogen)
arch-level-max = Maximum (stärkste Kompression)
arch-tar-store-hint = TAR bündelt Dateien ohne Kompression (nur Speichern).
arch-conflict-label = Falls vorhanden
arch-conflict-rename = Automatisch umbenennen
arch-conflict-overwrite = Überschreiben
arch-conflict-skip = Bei Konflikt abbrechen
arch-start-compress = Komprimieren
arch-minimize = Minimieren
arch-restore = Wiederherstellen
arch-compressing = Komprimiere
arch-extracting = Entpacke
arch-done-compress = Archiv erstellt
arch-done-extract = Entpacken abgeschlossen
arch-cancelled = Vorgang abgebrochen
arch-password-title = Passwortgeschütztes Archiv
arch-password-hint = Geben Sie das Passwort ein, um dieses Archiv zu entpacken.
arch-password-placeholder = Passwort...
arch-password-wrong = Falsches Passwort. Bitte versuchen Sie es erneut.
arch-password-submit = Entpacken
arch-password-toggle = Passwort anzeigen oder verbergen

# Duplikatsuche
dup-menu-action = Doppelte Dateien suchen...
dup-modal-title = Doppelte Dateien Suchen
dup-readonly-error = Die Duplikatsuche wird im Papierkorb oder in komprimierten Archiven nicht unterstützt.
dup-confirm-level-label = Bestätigungsstufe für Duplikate
dup-level-quick = 1. Schnell — Größe + Partielles MD5 (Vorauswahl)
dup-level-standard = 2. Standard — Größe + Partielles MD5 + Vollständiges MD5
dup-level-sha256 = 3. Sicher — Vollständiges MD5 + SHA-256-Prüfung
dup-level-byte = 4. Maximum — Vollständiges MD5 + Byte-für-Byte-Vergleich
dup-min-size-label = Mindestgröße
dup-min-size-any = Beliebig (≥ 1 B)
dup-min-size-1kb = ≥ 1 KB
dup-min-size-1mb = ≥ 1 MB
dup-min-size-10mb = ≥ 10 MB
dup-start-scan = Suche starten
dup-rescan = Neu scannen
dup-stop-scan = Stopp
dup-opt-subfolders = Unterordner einschließen
dup-opt-hidden = Versteckte einschließen
dup-opt-ignore-empty = Leere ignorieren (0 B)
dup-opt-ignore-hardlinks = Hardlinks ignorieren
dup-quick-warning = Der Schnellmodus vergleicht nur Anfang, Mitte und Ende. Zum dauerhaften Löschen werden Standard, Sicher oder Maximum empfohlen.
dup-quick-select-label = Schnellauswahl:
dup-keep-oldest = Älteste behalten
dup-keep-newest = Neueste behalten
dup-keep-shortest = Kürzester Pfad
dup-clear-selection = Leeren
dup-badge-oldest = Älteste
dup-badge-hardlink = Hardlink
dup-verify-partial = Partielles MD5 (ca.)
dup-verify-md5 = Vollst. MD5 ✓
dup-verify-sha256 = MD5 + SHA-256 ✓
dup-verify-byte = Byte-für-Byte 100% ✓
dup-stage-walking = Ordner durchsuchen
dup-stage-partial = Partielles MD5
dup-stage-full = Vollständiges MD5
dup-stage-sha256 = MD5 + SHA-256
dup-stage-byte = MD5 + Byte-für-Byte
dup-all-selected-warning = Achtung: ALLE Kopien von mindestens 1 Gruppe sind markiert! Entfernen Sie 1 Markierung, um das Original zu behalten.
dup-delete-permanent = Dauerhaft löschen
dup-move-to-trash = In Papierkorb verschieben
dup-confirm-perm-delete = Dauerhaftes Löschen bestätigen?
dup-empty-idle = Wählen Sie eine Bestätigungsstufe und klicken Sie auf Suche starten, um Duplikate zu finden.
dup-empty-none = Keine doppelten Dateien mit den aktuellen Filtern gefunden.
dup-group-prefix = Gruppe
dup-copies-word = Kopien
dup-each-word = je
dup-selected-word = markiert
dup-groups-found = Gruppen gefunden
dup-reclaimable = einsparbar
dup-footer-prefix = Duplikate
dup-cancelled = Duplikatsuche abgebrochen
dup-stale-skipped = Einige Dateien wurden geändert oder gelöscht und aus Sicherheitsgründen übersprungen.
dup-last-copy-protected = Löschen blockiert in Gruppe(n), deren letzte unmarkierte Kopie nicht mehr existiert.
dup-delete-done = Duplikate sicher entfernt
ctx-send-to = Senden an...
ctx-shortcut-desktop = Verknüpfung auf dem Desktop erstellen
ctx-shortcut-here-unix = Symbolischen Link hier erstellen
ctx-shortcut-here-win = Verknüpfung hier erstellen
shortcut-desktop-missing = Desktop-Ordner nicht gefunden.
filter-ext-placeholder = z. B. .exe, .zip...
shortcuts-created =
    { $count ->
        [one] Verknüpfung erstellt.
       *[other] { $count } Verknüpfungen erstellt.
    }
undo-action = Rückgängig (Strg+Z)
undo-trash-action = Löschen rückgängig machen (Strg+Z)
moved-to-trash-undo-hint = In den Papierkorb verschoben (Strg+Z zum Rückgängigmachen)
undo-nothing-in-trash = Element nicht mehr im Papierkorb zum Wiederherstellen.
undo-rename-done = Umbenennung rückgängig gemacht.
undo-restored-count =
    { $count ->
        [one] Aus dem Papierkorb wiederhergestellt.
       *[other] { $count } Elemente aus dem Papierkorb wiederhergestellt.
    }
mount-disk-image = Abbild einhängen
unmount-disk-image = Abbild aushängen
eject-safely = Sicher entfernen
image-mounted = Abbild erfolgreich eingehängt
drive-ejected = Laufwerk sicher entfernt
drive-free-label = frei
drive-click-to-mount = Zum Einhängen klicken
tip-split-view = Geteilte Ansicht (F3)
close-split-pane = Geteilte Ansicht schließen (F3)
drag-move-prefix = Verschieben
drag-copy-prefix = Kopieren
drag-ctrl-copy-hint = Strg = Kopieren
update-available-banner = Neue stabile Version verfügbar:
update-download-btn = Update herunterladen
update-ignore-btn = Diese Version ignorieren
update-check-btn = Nach Updates suchen
update-checking = Suche nach Updates...
update-up-to-date = Sie verwenden bereits die neueste stabile Version.
update-check-failed = Updates konnten derzeit nicht geprüft werden.
update-auto-check-label = Beim Start nach stabilen Updates suchen
update-open-releases = Release-Seite öffnen
tip-update-available = Neue stabile Version verfügbar — klicken, um die Release-Seite zu öffnen
recent-files = Zuletzt verwendet
prefs-recent-files-label = Zuletzt verwendete Dateien anzeigen
clear-recent-files = Verlauf löschen
remove-from-recent = Aus Verlauf entfernen
open-file-location = Dateipfad öffnen
open-target-location = Zielpfad öffnen
prefs-startup-location-label = Startordner beim Öffnen
startup-home = Benutzerordner
startup-last-visited = Zuletzt geöffneter Ordner
ntfs-object-name = Objektname:
ntfs-principals-label = Gruppen- oder Benutzernamen:
ntfs-edit-hint = Klicken Sie auf Bearbeiten, um Berechtigungen zu ändern.
ntfs-edit-active-hint = Zulassen oder Verweigern einzeln wählen (oder Vollzugriff).
ntfs-edit-btn = Bearbeiten...
ntfs-done-edit-btn = Bearbeitung beenden
ntfs-add-btn = Hinzufügen...
ntfs-remove-btn = Entfernen
ntfs-permissions-for = Berechtigungen für
ntfs-allow-col = Zulassen
ntfs-deny-col = Verweigern
ntfs-full-control = Vollzugriff
ntfs-list-folder = Ordnerinhalt auflisten
ntfs-delete-modify = Löschen / Ändern
ntfs-inherited-badge = Geerbt
ntfs-explicit-badge = Explizit
ntfs-owner-label = Besitzer
ntfs-inheritance-label = NTFS-Vererbung
ntfs-inheritance-on = Aktiviert (vom übergeordneten Ordner)
ntfs-inheritance-off = Deaktiviert (explizit)
ntfs-add-placeholder = z. B. Benutzer, Jeder, Administratoren...
ntfs-loading = NTFS-Berechtigungen werden geladen...
posix-owner-group-header = Besitzer & Gruppe (UID / GID)
posix-octal-symbolic-header = Oktal- & Symbolmodus
posix-presets-label = Vorlagen:
posix-preset-default = Standard
posix-preset-exec = Ausführbar
posix-preset-private = Privat
posix-preset-readonly = Schreibgeschützt
posix-preset-group = Gruppe
posix-preset-shared = Geteilt
posix-scope-col = Bereich
posix-summary-col = Kurzform
posix-others-sub = alle anderen Benutzer
posix-special-bits-header = Spezielle POSIX-Bits
posix-quick-action-header = Schnellzugriff
posix-recursive-header = Unterordner
posix-apply-recursive = Rekursiv auf Inhalt anwenden

analyze-space = Speicher analysieren
tip-analyze-space = Speicherplatzbelegung und große Dateien analysieren
disk-scan-eyebrow = Analyse läuft
disk-scan-title = Suche nach großen Dateien...
disk-scan-current-item = Aktuelles Element
disk-scan-files-checked = Geprüfte Dateien
disk-scan-data-checked = Geprüfte Daten
disk-scan-elapsed = Vergangene Zeit
disk-scan-hint = Ausgewählter Speicherort wird geprüft; Dauer hängt von der Dateianzahl ab
disk-results-title = Speicherplatzanalyse
disk-root-crumb = Ausgewählter Stamm
disk-treemap-title = Proportionale Karte (zum Erkunden klicken)
disk-top-consumers = Größte Verbraucher auf dieser Ebene
disk-by-type = Nach Dateityp
disk-mark-cleanup = Dateien oder Ordner zum Löschen markieren
disk-reanalyze = Erneut scannen
disk-clear-selection = Auswahl aufheben
disk-reclaim-suffix = werden freigegeben
disk-mode-files = Größte Dateien
disk-mode-folders = Ordner
disk-cat-iso = ISOs / Datenträger
disk-cat-build = Builds / Cache
disk-cat-archives = Archive
disk-cat-media = Medien
disk-cat-docs = Dokumente
disk-cat-other = Sonstige

search-name-label = Name / Pfad
search-containing-label = Inhalt suchen...
search-containing-placeholder = Text oder Regex in Dateien (überspringt Binärdateien)...
search-size-label = Größe
search-date-label = Geändert
search-type-label = Typ
search-hidden-label = Versteckt
search-streaming-hint = Streaming ohne Index • Ohne Binärdateien
search-any = Alle
search-today = Heute
search-last-7d = 7 Tage
search-last-30d = 30 Tage
search-type-code = Code
search-type-docs = Dokumente
search-type-images = Bilder
search-type-audio = Audio
search-type-video = Video
search-type-archives = Archive

cmd-palette-title = Befehlspalette
cmd-palette-placeholder = Aktion, Kategorie oder Kürzel eingeben...
cmd-palette-empty = Keine passenden Befehle gefunden.
cmd-palette-hint = ↑↓ navigieren • Enter ausführen • Esc schließen
cmd-cat-nav = Navigation
cmd-cat-search = Suche
cmd-cat-files = Dateien
cmd-cat-view = Ansicht
cmd-cat-tools = Werkzeuge
cmd-focus-quick-filter = Tab-Schnellfilter fokussieren
cmd-search-containing = Text in Dateien suchen...
cmd-clear-filters = Tab-Filter zurücksetzen
cmd-check-updates = Nach Updates suchen
cmd-toggle-theme = Helles/Dunkles Design wechseln
cmd-view-grid = Rasteransicht
cmd-view-list = Listenansicht

quick-jump-title = Schnellsprung (Quick Jump)
quick-jump-placeholder = Ordnername oder Pfad eingeben...
quick-jump-empty = Keine passenden Ordner gefunden.
quick-jump-hint = ↑↓ navigieren • Enter öffnen • Ctrl+Enter neuer Tab • Esc schließen
quick-jump-src-current = Aktuell
quick-jump-src-favorite = Favorit
quick-jump-src-recent = Zuletzt
quick-jump-src-subfolder = Unterordner
quick-jump-new-tab = Neuer Tab
