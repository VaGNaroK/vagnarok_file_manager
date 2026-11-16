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
view-columns = Colonnes
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
