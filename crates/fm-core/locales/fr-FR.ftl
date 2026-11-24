# Français (FR) — Locale pour Vagnarok File Manager

# Barre latérale et navigation
quick-access = ACCÈS RAPIDE
drives = DISQUES
tags-and-system = ÉTIQUETTES & SYSTÈME
documents = Documents
home = Accueil
desktop = Bureau
downloads = Téléchargements
pictures = Images
code-dev = Code / Dev
starred = Favoris
trash = Corbeille
preferences = Préférences

# En-tête et recherche
search-placeholder = Rechercher dans ce dossier...
deep-search = Sous-dossiers
deep-search-title = Recherche Profonde (Fuzzy)
deep-search-empty = Aucun fichier trouvé dans les sous-dossiers.
searching = Recherche dans les sous-dossiers...
no-active-tab = Aucun onglet actif
follow-system = Système (Auto)

# Barre d'actions, canevas et vues
new-item = + Nouveau
sort = Trier
hidden = Cachés
folders = DOSSIERS
files = FICHIERS
total-size = TAILLE DIRECTE
open-terminal = Terminal
pin-favorite = Épingler
unpin-favorite = Désépingler
remove-favorite = Retirer des favoris
filter-all = Tous
filter-ext = Filtrer
directories-only = Dossiers
archive-readonly = Archive compressée (lecture seule)
grid-group-label = Grouper les dossiers en Grille
dirs-first-label = Dossiers d'abord lors du tri
recursive-size-label = TAILLE TOTALE
empty-trash = Vider la corbeille
restore-item = Restaurer
confirm-empty-trash-msg = Supprimer définitivement tous les éléments de la corbeille ?
trash-emptied = Corbeille vidée.
trash-restored = Élément(s) restauré(s) à l'emplacement d'origine.
empty-subdir = Sous-dossier vide
loading = Chargement du dossier...
empty-dir = Dossier vide
go-back = Retour
retry = Réessayer
root-fs = Système
dir-badge = REP
file-badge = FIC

# Colonnes de vue détaillée
col-name = Nom
col-size = Taille
col-type = Type
col-perms = Permissions
col-modified = Modifié

# Inspecteur et QuickLook
inspector-title = Inspecteur
inspector-empty = Sélectionnez un élément pour voir ses métadonnées, permissions et hachage.
permissions = Permissions
modified = Modifié
created = Créé
checksum = SHA-256
mime-type = Type MIME
computing-hash = calcul en cours...
quicklook-hint = Espace : QuickLook
quicklook-title = QuickLook
close = Fermer
binary-preview-hint = Fichier binaire ou trop volumineux — aucun aperçu direct.
git-status-label = Statut Git

# Pied de page et transferts
selected-none = Sélection : Aucune
free-of = libres sur
clipboard-copy-tag = Copiés
clipboard-cut-tag = Coupés
transferring = Transfert en cours...

# Opérations sur fichiers et fenêtres
new-folder = Nouveau Dossier
new-file = Nouveau Fichier
rename = Renommer (F2)
copy = Copier (Ctrl+C)
cut = Couper (Ctrl+X)
paste = Coller (Ctrl+V)
move-to-trash = Mettre à la corbeille (Suppr)
delete-permanent = Supprimer définitivement (Maj+Suppr)
confirm = Confirmer
cancel = Annuler
create = Créer
name-label = Nom
invalid-name = Nom invalide.
confirm-trash-msg = Envoyer les éléments sélectionnés à la corbeille ?
confirm-delete-msg = Cette action supprime définitivement du disque et est irréversible.
conflict-title = Conflit de destination
conflict-msg = Certains éléments existent déjà à la destination. Que souhaitez-vous faire ?
conflict-rename = Conserver les deux (Renommer)
conflict-overwrite = Écraser
conflict-skip = Ignorer les existants
open-item = Ouvrir
open-in-new-tab = Ouvrir dans un nouvel onglet
copy-path = Copier le chemin
select-all = Tout sélectionner (Ctrl+A)
path-copied = Chemin copié
properties = Propriétés

# Préférences
prefs-title = Préférences
theme-label = Thème
lang-label = Langue
hero-card-label = Afficher la carte résumé du dossier
decorations-label = Barre de fenêtre
default-view-label = Vue par défaut
view-grid = Grille
view-list = Liste
window-bar-native = Barre Native
window-bar-csd = CSD Intégré

# Fenêtre Propriétés
location-label = Emplacement
copy-sha256 = Copier SHA-256
hash-copied = SHA-256 copié
properties-header-prefix = { "Propriétés : " }
prop-tab-general = Général
prop-tab-security = Sécurité
prop-tab-history = Historique
prop-tab-metadata = Métadonnées
prop-tab-permissions = Permissions
prop-kind-label = Type
prop-system-mime-label = Système / MIME
prop-accessed-label = Dernier accès
prop-quick-props = Propriétés Rapides
prop-readonly = Lecture seule
prop-hidden = Caché
prop-advanced-settings = Paramètres Avancés
prop-confirm = Confirmer
prop-apply = Appliquer
prop-open-default = Ouvrir par défaut
prop-owner = Propriétaire
prop-group = Groupe
prop-others = Autres
prop-octal-mode = Mode octal
prop-symlink-target = Cible du lien
prop-archive-notice = Lecture seule (dans une archive compressée)
prop-applied-notice = Propriétés appliquées.
prop-pending-changes = Modifications en attente
prop-calculating = Calcul en cours...
prop-sha-na-folders = Non applicable aux dossiers
prop-sha-computing = Calcul du hachage SHA-256...
yes-label = Oui
no-label = Non
ext-label = Extension
perm-read = Lecture
perm-write = Écriture
perm-execute = Exécution

# Types d'éléments
kind-folder = Dossier de Fichiers
kind-symlink-dir = Lien symbolique (Dossier)
kind-symlink-file = Lien symbolique (Fichier)
kind-special = Élément Spécial
kind-file = Fichier
kind-file-ext = Fichier (.{ $ext })

# Formatage pluralisé et paramétré
items-count = { $count ->
    [one] 1 élément
   *[other] { $count } éléments
}
folders-count = { $count ->
    [one] 1 dossier
   *[other] { $count } dossiers
}
files-count = { $count ->
    [one] 1 fichier
   *[other] { $count } fichiers
}
selected-multi = { $count ->
    [one] 1 élément ({ $size })
   *[other] { $count } éléments ({ $size })
}
exact-bytes = ({ $bytes } octets)
age-now = à l'instant
age-minutes = il y a { $m } min
age-hours = il y a { $h } h
age-days = il y a { $d } j
ts-today = Aujourd'hui, { $time }
ts-yesterday = Hier, { $time }

# Onglets Préférences et À propos
prefs-tab-general = Général
about-tab = À propos
about-subtitle = Gestionnaire de fichiers natif haute performance pour Linux et Windows
about-version-label = Version
about-developer-label = Développeur
about-developer-value = en développement
about-stage-label = Statut
about-stage-alpha = Alpha — Aperçu de développement
about-license-label = Licence
about-stack-label = Moteur / Stack
about-copy-info = Copier les infos
about-info-copied = Informations copiées

# Propriétés du dossier (contenu et espace disque)
prop-contents-label = Contenu
prop-free-space-label = Espace libre
hidden-items-count = { $count ->
    [one] 1 caché
   *[other] { $count } cachés
}

# Info-bulles (Tooltips)
tip-new-tab = Ouvrir un nouvel onglet (Ctrl+T)
tip-close-tab = Fermer l'onglet (Ctrl+W)
tip-language = Changer la langue de l'interface
tip-theme = Basculer entre thème clair et sombre
tip-back = Précédent (Alt+←)
tip-forward = Suivant (Alt+→)
tip-up = Dossier parent (Alt+↑)
tip-home = Aller au dossier personnel
tip-edit-path = Modifier le chemin directement (Ctrl+L)
tip-deep-search = Rechercher récursivement dans les sous-dossiers (Ctrl+Shift+F)
tip-view-grid = Vue Grille
tip-view-list = Vue Tableau détaillé
tip-new-item = Créer un nouveau dossier ou fichier
tip-group-folders = Regrouper les dossiers avant les fichiers
tip-zoom-out = Réduire la taille des cartes (Ctrl+-)
tip-zoom-reset = Réinitialiser la taille par défaut (Ctrl+0)
tip-zoom-in = Augmenter la taille des cartes (Ctrl++)
tip-sort = Changer le critère de tri (Nom, Taille, Type, Modifié)
tip-sort-dir = Inverser l'ordre croissant ou décroissant
tip-filter = Filtrer par dossier ou extension
tip-hidden = Afficher ou masquer les fichiers cachés (Ctrl+H)
tip-terminal = Ouvrir un terminal ici (Ctrl+Shift+T)
tip-favorite = Épingler ou retirer des favoris
tip-inspector = Afficher ou masquer l'Inspecteur (Alt+I)
tip-preferences = Ouvrir Préférences (clic droit pour menu rapide)
tip-toggle-sidebar = Réduire ou développer la barre latérale (Ctrl+B)

# Emplacements et montage de disques
music = Musique
videos = Vidéos
public-dir = Public
templates = Modèles
filesystem-root = Système de fichiers
mount-drive = Monter
unmount-drive = Démonter
unmounted-label = Non monté
refresh-drives = Actualiser les lecteurs
tip-expand-home = Développer ou réduire les dossiers personnels
open-as-root = Ouvrir en tant que root
root-mode-badge = Mode Root (Privilèges élevés)
grid-center-label = Alignement des éléments en Grille
grid-align-left = De gauche à droite
grid-align-center = Centré
tip-grid-alignment = Basculer l'alignement de la grille (Gauche à droite ou Centré)
tip-system-icons = Basculer entre les icônes par défaut et celles du système
perm-allow-exec = Autoriser l'exécution en tant que programme
open-with = Ouvrir avec...
prop-tab-open-with = Ouvrir avec
open-with-prompt = Sélectionnez une application pour ouvrir ce fichier et les autres du même type :
open-with-default-app = Application par défaut
open-with-recommended-apps = Applications recommandées
open-with-other-apps = Autres applications
open-with-custom-hint = Ou saisissez/sélectionnez une commande ou un exécutable personnalisé :
open-with-custom-placeholder = Ex. : mpv --no-terminal ou /chemin/programme
open-with-add-to-list = Ajouter à la liste
open-with-set-default = Définir par défaut
open-with-reset-default = Rétablir les paramètres système
open-with-open-once = Ouvrir maintenant
open-with-loading = Recherche des applications installées...
open-with-empty = Aucune application compatible trouvée.
open-with-saved = Application par défaut mise à jour.
open-with-reset-done = Paramètre système par défaut rétabli.
open-with-pick-exec = Sélectionner un fichier exécutable

# Renommage par Lots
br-title = Renommer par lots
br-menu-item = Renommer par lots...
br-clear-rules = Effacer les règles
br-rules-header = Que voulez-vous faire ?
br-preview-header = Aperçu en temps réel
br-col-status = État
br-col-original = Nom actuel
br-col-new = Nouveau nom
br-unchanged = (inchangé)
br-skipped-folder = (dossier ignoré)
br-conflict-internal = Conflit : nom cible en double
br-conflict-external = Conflit : existe déjà sur le disque
br-invalid-name = Nom invalide
br-replace-title = Rechercher et Remplacer
br-find-label = Rechercher
br-replace-label = Remplacer par
br-where-label = Où appliquer
br-scope-anywhere = Partout
br-scope-start-name = Début du nom uniquement
br-scope-end-name = Fin du nom uniquement
br-scope-word-start = Début de mot
br-scope-word-end = Fin de mot
br-scope-whole-word = Mot entier
br-match-case = Respecter la casse
br-first-only = 1re occurrence uniquement
br-advanced-regex = Options avancées (Regex)
br-use-regex = Utiliser une expression régulière (Regex)
br-remove-title = Supprimer Texte / Caractères
br-remove-first = Premiers car.
br-remove-last = Derniers car.
br-remove-text-label = Texte à retirer
br-remove-digits = Chiffres (0-9)
br-remove-symbols = Symboles
br-collapse-spaces = Espaces en trop
br-insert-title = Insérer du Texte
br-insert-text-label = Texte à insérer
br-insert-pos-label = Position
br-insert-start = Au début du nom
br-insert-end = À la fin du nom
br-insert-offset-start = Position (depuis le début)
br-insert-offset-end = Position (depuis la fin)
br-case-title = Majuscules / Minuscules
br-case-keep = Aucun changement
br-case-upper = TOUT EN MAJUSCULES
br-case-lower = tout en minuscules
br-case-title-words = Première Lettre De Chaque Mot
br-case-sentence = 1re lettre uniquement en majuscule
br-number-title = Numérotation Séquentielle
br-number-append = À la fin du nom
br-number-prepend = Au début du nom
br-number-replace = Remplacer tout le nom
br-number-start = Début
br-number-step = Pas
br-number-digits = Chiffres
br-number-sep = Séparateur
br-sort-label = Ordre
br-sort-selection = Ordre de sélection
br-sort-name-asc = Nom (A-Z)
br-sort-name-desc = Nom (Z-A)
br-sort-date-desc = Plus récents
br-sort-date-asc = Plus anciens
br-sort-size-asc = Plus petits
br-sort-size-desc = Plus grands
br-media-title = Infos Photo / Musique (EXIF & ID3)
br-media-exif-ymd = Date photo (AAAA-MM-JJ)
br-media-exif-datetime = Date et heure photo
br-media-exif-camera = Modèle d'appareil
br-media-id3-track = N° piste
br-media-id3-artist = Artiste
br-media-id3-album = Album
br-media-id3-title = Titre
br-media-id3-year = Année
br-media-loading = Lecture des tags EXIF/ID3...
br-ext-title = Extension du Fichier
br-ext-keep = Conserver l'originale
br-ext-lower = minuscules (.jpg)
br-ext-upper = MAJUSCULES (.JPG)
br-ext-replace = Changer pour
br-ext-remove = Supprimer l'extension
br-include-folders = Inclure les dossiers
br-apply = Appliquer
br-applying = Renommage...
br-undo-last = Annuler le dernier lot
br-readonly-error = Le renommage par lots n'est pas disponible dans la Corbeille ou les archives.

# Extraction et compression d'archives
arch-extract-here = Extraire ici
arch-extract-subdir = Extraire dans un sous-dossier
arch-compress-as = Compresser en...
arch-add-7z = Ajouter à l'archive .7z
arch-modal-title = Compresser en...
arch-output-name = Nom de l'archive
arch-destination = Dossier de destination
arch-format-label = Format / Extension
arch-level-label = Niveau de compression
arch-format-7z = .7z (7-Zip — Haute compression)
arch-format-zip = .zip (ZIP — Universel)
arch-format-tar = .tar (TAR — Sans compression)
arch-format-targz = .tar.gz (TAR.GZ — Compressé avec Gzip)
arch-level-store = Stocker (sans compression)
arch-level-min = Minimum (rapide)
arch-level-normal = Normal (équilibré)
arch-level-max = Maximum (meilleure compression)
arch-tar-store-hint = Le format TAR regroupe les fichiers sans compression (Stocker uniquement).
arch-conflict-label = Si le fichier existe
arch-conflict-rename = Renommer automatiquement
arch-conflict-overwrite = Écraser
arch-conflict-skip = Annuler si existant
arch-start-compress = Compresser
arch-minimize = Réduire
arch-restore = Restaurer
arch-compressing = Compression
arch-extracting = Extraction
arch-done-compress = Archive créée
arch-done-extract = Extraction terminée
arch-cancelled = Opération annulée
arch-password-title = Archive protégée par mot de passe
arch-password-hint = Saisissez le mot de passe pour extraire cette archive.
arch-password-placeholder = Mot de passe...
arch-password-wrong = Mot de passe incorrect. Veuillez réessayer.
arch-password-submit = Extraire
arch-password-toggle = Afficher ou masquer le mot de passe

# Recherche de fichiers en double
dup-menu-action = Rechercher les fichiers en double...
dup-modal-title = Rechercher les Fichiers en Double
dup-readonly-error = La recherche de doublons n'est pas prise en charge dans la Corbeille ou les archives.
dup-confirm-level-label = Niveau de confirmation des doublons
dup-level-quick = 1. Rapide — Taille + MD5 Partiel (Tri)
dup-level-standard = 2. Standard — Taille + MD5 Partiel + MD5 Complet
dup-level-sha256 = 3. Sûr — MD5 Complet + Confirmation SHA-256
dup-level-byte = 4. Maximum — MD5 Complet + Comparaison Octet par Octet
dup-min-size-label = Taille minimale
dup-min-size-any = Toute (≥ 1 o)
dup-min-size-1kb = ≥ 1 Ko
dup-min-size-1mb = ≥ 1 Mo
dup-min-size-10mb = ≥ 10 Mo
dup-start-scan = Lancer l'analyse
dup-rescan = Réanalyser
dup-stop-scan = Arrêter
dup-opt-subfolders = Inclure les sous-dossiers
dup-opt-hidden = Inclure les fichiers cachés
dup-opt-ignore-empty = Ignorer les vides (0 o)
dup-opt-ignore-hardlinks = Ignorer les liens durs
dup-quick-warning = Le mode Rapide compare uniquement début, milieu et fin. Pour une suppression définitive, Standard, Sûr ou Maximum est recommandé.
dup-quick-select-label = Sélection rapide :
dup-keep-oldest = Garder le plus ancien
dup-keep-newest = Garder le plus récent
dup-keep-shortest = Chemin le plus court
dup-clear-selection = Effacer
dup-badge-oldest = Plus ancien
dup-badge-hardlink = Lien dur
dup-verify-partial = MD5 Partiel (approx.)
dup-verify-md5 = MD5 Complet ✓
dup-verify-sha256 = MD5 + SHA-256 ✓
dup-verify-byte = Octet par Octet 100% ✓
dup-stage-walking = Parcours du dossier
dup-stage-partial = MD5 Partiel
dup-stage-full = MD5 Complet
dup-stage-sha256 = MD5 + SHA-256
dup-stage-byte = MD5 + Octet par Octet
dup-all-selected-warning = Attention : TOUTES les copies d'au moins 1 groupe sont sélectionnées ! Décochez 1 exemplaire pour conserver l'original.
dup-delete-permanent = Supprimer définitivement
dup-move-to-trash = Mettre à la Corbeille
dup-confirm-perm-delete = Confirmer la suppression définitive ?
dup-empty-idle = Choisissez le niveau de confirmation et cliquez sur Lancer l'analyse pour trouver les doublons.
dup-empty-none = Aucun fichier en double trouvé avec les filtres actuels.
dup-group-prefix = Groupe
dup-copies-word = copies
dup-each-word = chacun
dup-selected-word = sélectionnés
dup-groups-found = groupes trouvés
dup-reclaimable = récupérables
dup-footer-prefix = Doublons
dup-cancelled = Analyse des doublons annulée
dup-stale-skipped = Certains fichiers ont changé ou disparu sur le disque et ont été ignorés par sécurité.
dup-last-copy-protected = Suppression bloquée dans le(s) groupe(s) dont la dernière copie non cochée n'existe plus.
dup-delete-done = Doublons supprimés en toute sécurité
ctx-send-to = Envoyer vers...
ctx-shortcut-desktop = Créer un raccourci sur le Bureau
ctx-shortcut-here-unix = Créer un lien symbolique ici
ctx-shortcut-here-win = Créer un raccourci ici
shortcut-desktop-missing = Dossier Bureau introuvable.
filter-ext-placeholder = ex. : .exe, .zip...
shortcuts-created =
    { $count ->
        [one] Raccourci créé avec succès.
       *[other] { $count } raccourcis créés avec succès.
    }
undo-action = Annuler (Ctrl+Z)
undo-trash-action = Annuler la suppression (Ctrl+Z)
moved-to-trash-undo-hint = Déplacé vers la Corbeille (Ctrl+Z pour annuler)
undo-nothing-in-trash = Élément introuvable dans la Corbeille à restaurer.
undo-rename-done = Renommage annulé.
undo-restored-count =
    { $count ->
        [one] Restauré depuis la Corbeille.
       *[other] { $count } éléments restaurés depuis la Corbeille.
    }
mount-disk-image = Monter l'image disque
unmount-disk-image = Démonter l'image
eject-safely = Retirer en toute sécurité
image-mounted = Image disque montée
drive-ejected = Lecteur retiré en toute sécurité
drive-free-label = libres
drive-click-to-mount = Cliquer pour monter
tip-split-view = Vue scindée (F3)
close-split-pane = Fermer le panneau scindé (F3)
drag-move-prefix = Déplacer
drag-copy-prefix = Copier
drag-ctrl-copy-hint = Ctrl = Copier
update-available-banner = Nouvelle version stable disponible :
update-download-btn = Télécharger la mise à jour
update-ignore-btn = Ignorer cette version
update-check-btn = Vérifier les mises à jour
update-checking = Vérification des mises à jour...
update-up-to-date = Vous utilisez déjà la dernière version stable.
update-check-failed = Impossible de vérifier les mises à jour pour le moment.
update-auto-check-label = Vérifier les mises à jour stables au démarrage
update-open-releases = Ouvrir la page de la version
tip-update-available = Nouvelle version stable disponible — cliquez pour ouvrir la page de téléchargement
recent-files = Récents
prefs-recent-files-label = Afficher les fichiers récents
clear-recent-files = Effacer les récents
remove-from-recent = Retirer des récents
open-file-location = Ouvrir l'emplacement du fichier
open-target-location = Ouvrir l'emplacement de la cible
prefs-startup-location-label = Dossier de démarrage
startup-home = Dossier personnel
startup-last-visited = Dernier dossier ouvert
ntfs-object-name = Nom de l'objet :
ntfs-principals-label = Noms de groupes ou d'utilisateurs :
ntfs-edit-hint = Pour modifier les autorisations, cliquez sur Modifier.
ntfs-edit-active-hint = Cochez Autoriser ou Refuser individuellement (ou Contrôle total).
ntfs-edit-btn = Modifier...
ntfs-done-edit-btn = Terminer
ntfs-add-btn = Ajouter...
ntfs-remove-btn = Supprimer
ntfs-permissions-for = Autorisations pour
ntfs-allow-col = Autoriser
ntfs-deny-col = Refuser
ntfs-full-control = Contrôle total
ntfs-list-folder = Affichage du contenu du dossier
ntfs-delete-modify = Supprimer / Modifier
ntfs-inherited-badge = Hérité
ntfs-explicit-badge = Explicite
ntfs-owner-label = Propriétaire
ntfs-inheritance-label = Héritage NTFS
ntfs-inheritance-on = Activé (hérité du parent)
ntfs-inheritance-off = Désactivé (explicites)
ntfs-add-placeholder = Ex. : Utilisateurs, Tout le monde...
ntfs-loading = Chargement des autorisations NTFS...
posix-owner-group-header = Propriétaire et Groupe (UID / GID)
posix-octal-symbolic-header = Mode Octal et Symbolique
posix-presets-label = Préréglages :
posix-preset-default = Par défaut
posix-preset-exec = Exécutable
posix-preset-private = Privé
posix-preset-readonly = Lecture seule
posix-preset-group = Groupe
posix-preset-shared = Partagé
posix-scope-col = Portée
posix-summary-col = Résumé
posix-others-sub = autres utilisateurs
posix-special-bits-header = Bits Spéciaux POSIX
posix-quick-action-header = Raccourci
posix-recursive-header = Sous-dossiers
posix-apply-recursive = Appliquer récursivement au contenu
