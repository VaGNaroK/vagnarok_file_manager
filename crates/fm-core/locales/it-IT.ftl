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
view-columns = Colonne
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
tip-view-columns = Vista a Colonne (Miller)
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
