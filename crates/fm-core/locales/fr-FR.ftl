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
tip-view-columns = Vue Colonnes (Miller)
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
