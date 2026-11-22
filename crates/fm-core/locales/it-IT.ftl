# Italiano (IT) — Locale per Vagnarok File Manager

# Barra laterale e navigazione
quick-access = ACCESSO RAPIDO
drives = UNITÀ
tags-and-system = TAG E SISTEMA
documents = Documenti
home = Home
desktop = Scrivania
downloads = Scaricati
pictures = Immagini
code-dev = Codice / Dev
starred = Preferiti
trash = Cestino
preferences = Preferenze

# Intestazione e ricerca
search-placeholder = Cerca in questa cartella...
deep-search = Sottocartelle
deep-search-title = Ricerca Profonda (Fuzzy)
deep-search-empty = Nessun file trovato nelle sottocartelle.
searching = Ricerca nelle sottocartelle...
no-active-tab = Nessuna scheda attiva
follow-system = Sistema (Auto)

# Barra azioni, canvas e viste
new-item = + Nuovo
sort = Ordina
hidden = Nascosti
folders = CARTELLE
files = FILE
total-size = DIMENSIONE DIRETTA
open-terminal = Terminale
pin-favorite = Fissa
unpin-favorite = Rimuovi fissaggio
remove-favorite = Rimuovi dai preferiti
filter-all = Tutti
filter-ext = Filtra
directories-only = Cartelle
archive-readonly = Archivio compresso (sola lettura)
grid-group-label = Raggruppa cartelle nella Griglia
dirs-first-label = Cartelle prima nell'ordinamento
recursive-size-label = DIMENSIONE TOTALE
empty-trash = Svuota cestino
restore-item = Ripristina
confirm-empty-trash-msg = Eliminare definitivamente tutti gli elementi nel cestino di sistema?
trash-emptied = Cestino svuotato.
trash-restored = Elementi ripristinati nella posizione originale.
empty-subdir = Sottocartella vuota
loading = Caricamento cartella...
empty-dir = Cartella vuota
go-back = Indietro
retry = Riprova
root-fs = Sistema
dir-badge = DIR
file-badge = FILE

# Colonne tabella dettagli
col-name = Nome
col-size = Dimensione
col-type = Tipo
col-perms = Permessi
col-modified = Modificato

# Ispettore e QuickLook
inspector-title = Ispettore
inspector-empty = Seleziona un elemento per vedere metadati, permessi e checksum.
permissions = Permessi
modified = Modificato
created = Creato
checksum = SHA-256
mime-type = Tipo MIME
computing-hash = calcolo in corso...
quicklook-hint = Spazio: QuickLook
quicklook-title = QuickLook
close = Chiudi
binary-preview-hint = File binario o troppo grande — anteprima diretta non disponibile.
git-status-label = Stato Git

# Piè di pagina e trasferimenti
selected-none = Selezionato: Nessuno
free-of = liberi su
clipboard-copy-tag = Copiati
clipboard-cut-tag = Tagliati
transferring = Trasferimento...

# Operazioni sui file e finestre
new-folder = Nuova Cartella
new-file = Nuovo File
rename = Rinomina (F2)
copy = Copia (Ctrl+C)
cut = Taglia (Ctrl+X)
paste = Incolla (Ctrl+V)
move-to-trash = Sposta nel Cestino (Canc)
delete-permanent = Elimina definitivamente (Shift+Canc)
confirm = Conferma
cancel = Annulla
create = Crea
name-label = Nome
invalid-name = Nome non valido.
confirm-trash-msg = Inviare gli elementi selezionati nel cestino di sistema?
confirm-delete-msg = Questa azione elimina definitivamente dal disco e non può essere annullata.
conflict-title = Conflitto di destinazione
conflict-msg = Alcuni elementi esistono già nella destinazione. Come procedere?
conflict-rename = Mantieni entrambi (Rinomina)
conflict-overwrite = Sovrascrivi
conflict-skip = Salta esistenti
open-item = Apri
open-in-new-tab = Apri in una nuova scheda
copy-path = Copia percorso
select-all = Seleziona tutto (Ctrl+A)
path-copied = Percorso copiato
properties = Proprietà

# Preferenze
prefs-title = Preferenze
theme-label = Tema
lang-label = Lingua
hero-card-label = Mostra scheda riepilogo cartella
decorations-label = Barra finestra
default-view-label = Vista predefinita
view-grid = Griglia
view-list = Elenco
window-bar-native = Barra Nativa
window-bar-csd = CSD Integrato

# Finestra Proprietà
location-label = Percorso
copy-sha256 = Copia SHA-256
hash-copied = SHA-256 copiato
properties-header-prefix = { "Proprietà: " }
prop-tab-general = Generale
prop-tab-security = Sicurezza
prop-tab-history = Cronologia
prop-tab-metadata = Metadati
prop-tab-permissions = Permessi
prop-kind-label = Tipo
prop-system-mime-label = Sistema / MIME
prop-accessed-label = Ultimo accesso
prop-quick-props = Proprietà Rapide
prop-readonly = Sola lettura
prop-hidden = Nascosto
prop-advanced-settings = Impostazioni Avanzate
prop-confirm = Conferma
prop-apply = Applica
prop-open-default = Apri predefinito
prop-owner = Proprietario
prop-group = Gruppo
prop-others = Altri
prop-octal-mode = Modalità ottale
prop-symlink-target = Destinazione symlink
prop-archive-notice = Sola lettura (dentro archivio compresso)
prop-applied-notice = Proprietà applicate.
prop-pending-changes = Modifiche in sospeso
prop-calculating = Calcolo in corso...
prop-sha-na-folders = Non applicabile alle cartelle
prop-sha-computing = Calcolo hash SHA-256...
yes-label = Sì
no-label = No
ext-label = Estensione
perm-read = Lettura
perm-write = Scrittura
perm-execute = Esecuzione

# Tipi di elementi
kind-folder = Cartella di File
kind-symlink-dir = Collegamento simbolico (Cartella)
kind-symlink-file = Collegamento simbolico (File)
kind-special = Voce Speciale
kind-file = File
kind-file-ext = File (.{ $ext })

# Formattazione pluralizzata e parametrizzata
items-count = { $count ->
    [one] 1 elemento
   *[other] { $count } elementi
}
folders-count = { $count ->
    [one] 1 cartella
   *[other] { $count } cartelle
}
files-count = { $count ->
    [one] 1 file
   *[other] { $count } file
}
selected-multi = { $count ->
    [one] 1 elemento ({ $size })
   *[other] { $count } elementi ({ $size })
}
exact-bytes = ({ $bytes } byte)
age-now = ora
age-minutes = { $m } min fa
age-hours = { $h } h fa
age-days = { $d } g fa
ts-today = Oggi, { $time }
ts-yesterday = Ieri, { $time }

# Schede Preferenze e Informazioni
prefs-tab-general = Generale
about-tab = Informazioni
about-subtitle = Gestore di file nativo ad alte prestazioni per Linux e Windows
about-version-label = Versione
about-developer-label = Sviluppatore
about-developer-value = in sviluppo
about-stage-label = Stadio
about-stage-alpha = Alpha — Anteprima di sviluppo
about-license-label = Licenza
about-stack-label = Motore / Stack
about-copy-info = Copia info
about-info-copied = Informazioni copiate

# Proprietà cartella (contenuto e spazio su disco)
prop-contents-label = Contenuto
prop-free-space-label = Spazio libero
hidden-items-count = { $count ->
    [one] 1 nascosto
   *[other] { $count } nascosti
}

# Suggerimenti (Tooltip)
tip-new-tab = Apri nuova scheda (Ctrl+T)
tip-close-tab = Chiudi scheda (Ctrl+W)
tip-language = Cambia lingua dell'interfaccia
tip-theme = Alterna tema chiaro/scuro
tip-back = Indietro (Alt+←)
tip-forward = Avanti (Alt+→)
tip-up = Cartella superiore (Alt+↑)
tip-home = Vai alla cartella Home
tip-edit-path = Modifica percorso direttamente (Ctrl+L)
tip-deep-search = Cerca ricorsivamente nelle sottocartelle (Ctrl+Shift+F)
tip-view-grid = Vista Griglia
tip-view-list = Vista Tabella Dettagli
tip-new-item = Crea nuova cartella o file
tip-group-folders = Raggruppa le cartelle prima dei file
tip-zoom-out = Riduci dimensione schede (Ctrl+-)
tip-zoom-reset = Ripristina dimensione predefinita (Ctrl+0)
tip-zoom-in = Aumenta dimensione schede (Ctrl++)
tip-sort = Cambia criterio di ordinamento (Nome, Dimensione, Tipo, Modificato)
tip-sort-dir = Alterna ordine crescente o decrescente
tip-filter = Filtra elementi per cartella o estensione
tip-hidden = Mostra o nascondi file nascosti (Ctrl+H)
tip-terminal = Apri terminale in questa cartella (Ctrl+Shift+T)
tip-favorite = Aggiungi o rimuovi cartella dai preferiti
tip-inspector = Mostra o nascondi pannello Ispettore (Alt+I)
tip-preferences = Apri Preferenze (clic destro per menu rapido)
tip-toggle-sidebar = Comprimi o espandi barra laterale (Ctrl+B)

# Risorse utente e montaggio dischi
music = Musica
videos = Video
public-dir = Pubblici
templates = Modelli
filesystem-root = File system
mount-drive = Monta
unmount-drive = Smonta
unmounted-label = Smontato
refresh-drives = Aggiorna unità
tip-expand-home = Espandi o comprimi cartelle della Home
open-as-root = Apri come root
root-mode-badge = Modalità Root (Privilegi elevati)
grid-center-label = Allineamento elementi nella Griglia
grid-align-left = Da sinistra a destra
grid-align-center = Centrato
tip-grid-alignment = Cambia allineamento griglia (Sinistra a destra o Centrato)
tip-system-icons = Alterna tra icone predefinite e icone di sistema
perm-allow-exec = Abilita esecuzione come programma
open-with = Apri con...
prop-tab-open-with = Apri con
open-with-prompt = Seleziona un'applicazione per aprire questo file e altri dello stesso tipo:
open-with-default-app = Applicazione predefinita
open-with-recommended-apps = Applicazioni consigliate
open-with-other-apps = Altre applicazioni
open-with-custom-hint = Oppure inserisci/seleziona un comando o eseguibile personalizzato:
open-with-custom-placeholder = Es.: mpv --no-terminal o /percorso/programma
open-with-add-to-list = Aggiungi alla lista
open-with-set-default = Imposta come predefinita
open-with-reset-default = Ripristina predefiniti di sistema
open-with-open-once = Apri ora
open-with-loading = Ricerca applicazioni installate...
open-with-empty = Nessuna applicazione compatibile trovata.
open-with-saved = Applicazione predefinita aggiornata.
open-with-reset-done = Predefinito di sistema ripristinato.
open-with-pick-exec = Seleziona file eseguibile

# Rinomina Multipla
br-title = Rinomina in blocco
br-menu-item = Rinomina in blocco...
br-clear-rules = Cancella regole
br-rules-header = Cosa vuoi fare?
br-preview-header = Anteprima in tempo reale
br-col-status = Stato
br-col-original = Nome attuale
br-col-new = Nuovo nome
br-unchanged = (nessuna modifica)
br-skipped-folder = (cartella ignorata)
br-conflict-internal = Conflitto: nome finale duplicato
br-conflict-external = Conflitto: già presente su disco
br-invalid-name = Nome non valido
br-replace-title = Trova e Sostituisci
br-find-label = Trova
br-replace-label = Sostituisci con
br-where-label = Dove applicare
br-scope-anywhere = Ovunque
br-scope-start-name = Solo inizio nome
br-scope-end-name = Solo fine nome
br-scope-word-start = Inizio parola
br-scope-word-end = Fine parola
br-scope-whole-word = Parola intera
br-match-case = Maiuscole/minuscole
br-first-only = Solo 1ª occorrenza
br-advanced-regex = Opzioni avanzate (Regex)
br-use-regex = Usa espressione regolare (Regex)
br-remove-title = Rimuovi Testo / Caratteri
br-remove-first = Primi car.
br-remove-last = Ultimi car.
br-remove-text-label = Testo da rimuovere
br-remove-digits = Numeri (0-9)
br-remove-symbols = Simboli
br-collapse-spaces = Spazi extra
br-insert-title = Inserisci Testo
br-insert-text-label = Testo da inserire
br-insert-pos-label = Posizione
br-insert-start = All'inizio del nome
br-insert-end = Alla fine del nome
br-insert-offset-start = Alla posizione (dall'inizio)
br-insert-offset-end = Alla posizione (dalla fine)
br-case-title = Maiuscole / Minuscole
br-case-keep = Nessuna modifica
br-case-upper = TUTTO MAIUSCOLO
br-case-lower = tutto minuscolo
br-case-title-words = Prima Lettera Di Ogni Parola
br-case-sentence = Solo 1ª lettera maiuscola
br-number-title = Numerazione Sequenziale
br-number-append = Alla fine del nome
br-number-prepend = All'inizio del nome
br-number-replace = Sostituisci tutto il nome
br-number-start = Inizio
br-number-step = Passo
br-number-digits = Cifre
br-number-sep = Separatore
br-sort-label = Ordine
br-sort-selection = Ordine selezione
br-sort-name-asc = Nome (A-Z)
br-sort-name-desc = Nome (Z-A)
br-sort-date-desc = Più recenti
br-sort-date-asc = Meno recenti
br-sort-size-asc = Più piccoli
br-sort-size-desc = Più grandi
br-media-title = Dati Foto / Musica (EXIF e ID3)
br-media-exif-ymd = Data foto (AAAA-MM-GG)
br-media-exif-datetime = Data e ora foto
br-media-exif-camera = Modello fotocamera
br-media-id3-track = N. traccia
br-media-id3-artist = Artista
br-media-id3-album = Album
br-media-id3-title = Titolo brano
br-media-id3-year = Anno
br-media-loading = Lettura tag EXIF/ID3...
br-ext-title = Estensione File
br-ext-keep = Mantieni originale
br-ext-lower = minuscolo (.jpg)
br-ext-upper = MAIUSCOLO (.JPG)
br-ext-replace = Cambia in
br-ext-remove = Rimuovi estensione
br-include-folders = Includi cartelle
br-apply = Applica
br-applying = Rinomina...
br-undo-last = Annulla ultima rinomina
br-readonly-error = La rinomina in blocco non è supportata nel Cestino o negli archivi.

# Estrazione e compressione archivi
arch-extract-here = Estrai qui
arch-extract-subdir = Estrai in sottocartella
arch-compress-as = Comprimi come...
arch-add-7z = Aggiungi ad archivio .7z
arch-modal-title = Comprimi come...
arch-output-name = Nome archivio
arch-destination = Cartella di destinazione
arch-format-label = Formato / Estensione
arch-level-label = Livello di compressione
arch-format-7z = .7z (7-Zip — Alta compressione)
arch-format-zip = .zip (ZIP — Universale)
arch-format-tar = .tar (TAR — Senza compressione)
arch-format-targz = .tar.gz (TAR.GZ — Compresso con Gzip)
arch-level-store = Memorizza (nessuna compressione)
arch-level-min = Minimo (veloce)
arch-level-normal = Normale (bilanciato)
arch-level-max = Massimo (massima compressione)
arch-tar-store-hint = Il formato TAR raggruppa i file senza compressione (solo Memorizza).
arch-conflict-label = Se già esistente
arch-conflict-rename = Rinomina automaticamente
arch-conflict-overwrite = Sovrascrivi
arch-conflict-skip = Annulla se esiste
arch-start-compress = Comprimi
arch-minimize = Riduci a icona
arch-restore = Ripristina
arch-compressing = Compressione
arch-extracting = Estrazione
arch-done-compress = Archivio creato
arch-done-extract = Estrazione completata
arch-cancelled = Operazione annullata
arch-password-title = Archivio protetto da password
arch-password-hint = Inserisci la password per estrarre questo archivio.
arch-password-placeholder = Password...
arch-password-wrong = Password errata. Riprova.
arch-password-submit = Estrai
arch-password-toggle = Mostra o nascondi password

# Ricerca file duplicati
dup-menu-action = Cerca file duplicati...
dup-modal-title = Cerca File Duplicati
dup-readonly-error = La ricerca di duplicati non è supportata nel Cestino o negli archivi compressi.
dup-confirm-level-label = Livello di conferma duplicati
dup-level-quick = 1. Rapido — Dimensione + MD5 Parziale (Screening)
dup-level-standard = 2. Standard — Dimensione + MD5 Parziale + MD5 Completo
dup-level-sha256 = 3. Sicuro — MD5 Completo + Conferma SHA-256
dup-level-byte = 4. Massimo — MD5 Completo + Confronto Byte per Byte
dup-min-size-label = Dimensione minima
dup-min-size-any = Qualsiasi (≥ 1 B)
dup-min-size-1kb = ≥ 1 KB
dup-min-size-1mb = ≥ 1 MB
dup-min-size-10mb = ≥ 10 MB
dup-start-scan = Avvia scansione
dup-rescan = Riscansiona
dup-stop-scan = Ferma
dup-opt-subfolders = Includi sottocartelle
dup-opt-hidden = Includi nascosti
dup-opt-ignore-empty = Ignora vuoti (0 B)
dup-opt-ignore-hardlinks = Ignora hardlink
dup-quick-warning = La modalità Rapida confronta solo inizio, centro e fine dei file. Per l'eliminazione definitiva si consiglia Standard, Sicuro o Massimo.
dup-quick-select-label = Selezione rapida:
dup-keep-oldest = Mantieni più vecchio
dup-keep-newest = Mantieni più recente
dup-keep-shortest = Percorso più corto
dup-clear-selection = Pulisci
dup-badge-oldest = Più vecchio
dup-badge-hardlink = Hardlink
dup-verify-partial = MD5 Parziale (approx.)
dup-verify-md5 = MD5 Completo ✓
dup-verify-sha256 = MD5 + SHA-256 ✓
dup-verify-byte = Byte per Byte 100% ✓
dup-stage-walking = Scansione cartella
dup-stage-partial = MD5 Parziale
dup-stage-full = MD5 Completo
dup-stage-sha256 = MD5 + SHA-256
dup-stage-byte = MD5 + Byte per Byte
dup-all-selected-warning = Attenzione: hai selezionato TUTTE le copie di almeno 1 gruppo! Deseleziona 1 copia per non perdere il file originale.
dup-delete-permanent = Elimina definitivamente
dup-move-to-trash = Sposta nel Cestino
dup-confirm-perm-delete = Confermare l'eliminazione definitiva?
dup-empty-idle = Scegli il livello di conferma e fai clic su Avvia scansione per trovare i duplicati.
dup-empty-none = Nessun file duplicato trovato con i filtri correnti.
dup-group-prefix = Gruppo
dup-copies-word = copie
dup-each-word = ciascuno
dup-selected-word = selezionati
dup-groups-found = gruppi trovati
dup-reclaimable = recuperabili
dup-footer-prefix = Duplicati
dup-cancelled = Scansione duplicati annullata
dup-stale-skipped = Alcuni file sono stati modificati o rimossi su disco e sono stati saltati per sicurezza.
dup-last-copy-protected = Eliminazione bloccata nei gruppi la cui ultima copia non selezionata non esiste più su disco.
dup-delete-done = Duplicati rimossi in sicurezza
ctx-send-to = Invia a...
ctx-shortcut-desktop = Crea collegamento sul Desktop
ctx-shortcut-here-unix = Crea collegamento simbolico qui
ctx-shortcut-here-win = Crea collegamento qui
shortcut-desktop-missing = Cartella Desktop non trovata.
filter-ext-placeholder = es.: .exe, .zip...
shortcuts-created =
    { $count ->
        [one] Collegamento creato con successo.
       *[other] { $count } collegamenti creati con successo.
    }
undo-action = Annulla (Ctrl+Z)
undo-trash-action = Annulla eliminazione (Ctrl+Z)
moved-to-trash-undo-hint = Spostato nel Cestino (Ctrl+Z per annullare)
undo-nothing-in-trash = Elemento non più presente nel Cestino da ripristinare.
undo-rename-done = Rinomina annullata.
undo-restored-count =
    { $count ->
        [one] Ripristinato dal Cestino.
       *[other] { $count } elementi ripristinati dal Cestino.
    }
mount-disk-image = Monta immagine disco
unmount-disk-image = Smonta immagine
eject-safely = Rimozione sicura
image-mounted = Immagine disco montata
drive-ejected = Unità rimossa in sicurezza
drive-free-label = liberi
drive-click-to-mount = Fai clic per montare
tip-split-view = Vista divisa (F3)
close-split-pane = Chiudi pannello diviso (F3)
drag-move-prefix = Sposta
drag-copy-prefix = Copia
drag-ctrl-copy-hint = Ctrl = Copia
update-available-banner = Nuova versione stabile disponibile:
update-download-btn = Scarica aggiornamento
update-ignore-btn = Ignora questa versione
update-check-btn = Controlla aggiornamenti
update-checking = Controllo aggiornamenti...
update-up-to-date = Stai già usando l'ultima versione stabile.
update-check-failed = Impossibile controllare gli aggiornamenti ora.
update-auto-check-label = Controlla aggiornamenti stabili all'avvio
update-open-releases = Apri pagina della release
tip-update-available = Nuova versione stabile disponibile — clicca per aprire la pagina della release
recent-files = Recenti
prefs-recent-files-label = Mostra file recenti
clear-recent-files = Svuota recenti
remove-from-recent = Rimuovi dai recenti
open-file-location = Apri percorso file
prefs-startup-location-label = Cartella iniziale all'avvio
startup-home = Cartella utente
startup-last-visited = Ultima cartella aperta
ntfs-object-name = Nome oggetto:
ntfs-principals-label = Nomi di gruppi o utenti:
ntfs-edit-hint = Per modificare le autorizzazioni, fai clic su Modifica.
ntfs-edit-active-hint = Seleziona Consenti o Nega singolarmente (o Controllo completo).
ntfs-edit-btn = Modifica...
ntfs-done-edit-btn = Fine modifica
ntfs-add-btn = Aggiungi...
ntfs-remove-btn = Rimuovi
ntfs-permissions-for = Autorizzazioni per
ntfs-allow-col = Consenti
ntfs-deny-col = Nega
ntfs-full-control = Controllo completo
ntfs-list-folder = Visualizzazione contenuto cartella
ntfs-delete-modify = Elimina / Modifica
ntfs-inherited-badge = Ereditato
ntfs-explicit-badge = Esplicito
ntfs-owner-label = Proprietario
ntfs-inheritance-label = Ereditarietà NTFS
ntfs-inheritance-on = Attivata (ereditata dal padre)
ntfs-inheritance-off = Disattivata (esplicite)
ntfs-add-placeholder = Es.: Users, Everyone, Administrators...
ntfs-loading = Caricamento autorizzazioni NTFS...
posix-owner-group-header = Proprietario e Gruppo (UID / GID)
posix-octal-symbolic-header = Modalità Ottale e Simbolica
posix-presets-label = Predefiniti:
posix-preset-default = Predefinito
posix-preset-exec = Eseguibile
posix-preset-private = Privato
posix-preset-readonly = Sola lettura
posix-preset-group = Gruppo
posix-preset-shared = Condivisa
posix-scope-col = Ambito
posix-summary-col = Riepilogo
posix-others-sub = altri utenti
posix-special-bits-header = Bit Speciali POSIX
posix-quick-action-header = Azione Rapida
posix-recursive-header = Sottocartelle
posix-apply-recursive = Applica ricorsivamente al contenuto
