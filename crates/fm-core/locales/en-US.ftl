# English (US) — Canonical Locale for Vagnarok File Manager

# Sidebar & navigation
quick-access = QUICK ACCESS
drives = DRIVES
tags-and-system = TAGS & SYSTEM
documents = Documents
home = Home
desktop = Desktop
downloads = Downloads
pictures = Pictures
code-dev = Code / Dev
starred = Favorites
trash = Trash
preferences = Preferences

# Header & Search
search-placeholder = Search in directory...
deep-search = Subfolders
deep-search-title = Deep Search (Fuzzy)
deep-search-empty = No matching files found in subfolders.
searching = Searching subfolders...
no-active-tab = No active tab
follow-system = System (Auto)

# Action bar, canvas & views
new-item = + New
sort = Sort
hidden = Hidden
folders = FOLDERS
files = FILES
total-size = DIRECT SIZE
open-terminal = Terminal
pin-favorite = Pin
unpin-favorite = Unpin
remove-favorite = Remove from favorites
filter-all = All
filter-ext = Filter
directories-only = Folders
archive-readonly = Compressed archive (read-only)
grid-group-label = Group folders in Grid
dirs-first-label = Folders first when sorting
recursive-size-label = TOTAL SIZE
empty-trash = Empty trash
restore-item = Restore
confirm-empty-trash-msg = Permanently delete every item currently in the system trash?
trash-emptied = Trash emptied.
trash-restored = Restored item(s) to original location.
empty-subdir = Empty subfolder
loading = Loading directory...
empty-dir = Empty folder
go-back = Go back
retry = Retry
root-fs = System
dir-badge = DIR
file-badge = FILE

# Detail table columns
col-name = Name
col-size = Size
col-type = Type
col-perms = Permissions
col-modified = Modified

# Inspector & QuickLook
inspector-title = Inspector
inspector-empty = Select an item to view metadata, permissions, and checksum.
permissions = Permissions
modified = Modified
created = Created
checksum = SHA-256
mime-type = MIME Type
computing-hash = computing...
quicklook-hint = Space: QuickLook
quicklook-title = QuickLook
close = Close
binary-preview-hint = Binary or oversized file — no direct preview available.
git-status-label = Git Status

# Footer & transfer
selected-none = Selected: None
free-of = free of
clipboard-copy-tag = Copied
clipboard-cut-tag = Cut
transferring = Transferring...

# File operations & modals
new-folder = New Folder
new-file = New File
rename = Rename (F2)
copy = Copy (Ctrl+C)
cut = Cut (Ctrl+X)
paste = Paste (Ctrl+V)
move-to-trash = Move to Trash (Del)
delete-permanent = Delete Permanently (Shift+Del)
confirm = Confirm
cancel = Cancel
create = Create
name-label = Name
invalid-name = Invalid name.
confirm-trash-msg = Send the selected items to the system trash?
confirm-delete-msg = This permanently deletes items from disk and cannot be undone.
conflict-title = Destination Conflict
conflict-msg = Some items already exist at the destination. How would you like to proceed?
conflict-rename = Keep both (Rename)
conflict-overwrite = Overwrite
conflict-skip = Skip existing
open-item = Open
open-in-new-tab = Open in new tab
copy-path = Copy path
select-all = Select all (Ctrl+A)
path-copied = Path copied
properties = Properties

# Preferences
prefs-title = Preferences
theme-label = Theme
lang-label = Language
hero-card-label = Show folder summary card
decorations-label = Window bar
default-view-label = Default view mode
view-grid = Grid
view-list = List
view-columns = Columns
window-bar-native = Native WM
window-bar-csd = Integrated CSD

# Properties window
location-label = Location
copy-sha256 = Copy SHA-256
hash-copied = SHA-256 copied
properties-header-prefix = { "Properties: " }
prop-tab-general = General
prop-tab-security = Security
prop-tab-history = History
prop-tab-metadata = Metadata
prop-tab-permissions = Permissions
prop-kind-label = Type
prop-system-mime-label = System / MIME
prop-accessed-label = Last accessed
prop-quick-props = Quick Properties
prop-readonly = Read-only
prop-hidden = Hidden
prop-advanced-settings = Advanced Settings
prop-confirm = Confirm
prop-apply = Apply
prop-open-default = Open default
prop-owner = Owner
prop-group = Group
prop-others = Others
prop-octal-mode = Octal mode
prop-symlink-target = Symlink target
prop-archive-notice = Read-only (inside compressed archive)
prop-applied-notice = Properties applied.
prop-pending-changes = Pending changes
prop-calculating = Calculating...
prop-sha-na-folders = Not applicable to folders
prop-sha-computing = Computing SHA-256 hash...
yes-label = Yes
no-label = No
ext-label = Extension
perm-read = Read
perm-write = Write
perm-execute = Execute

# Entry kinds
kind-folder = File Folder
kind-symlink-dir = Symbolic link (Folder)
kind-symlink-file = Symbolic link (File)
kind-special = Special Entry
kind-file = File
kind-file-ext = File (.{ $ext })

# Pluralized & parameterized formatting
items-count = { $count ->
    [one] 1 item
   *[other] { $count } items
}
folders-count = { $count ->
    [one] 1 folder
   *[other] { $count } folders
}
files-count = { $count ->
    [one] 1 file
   *[other] { $count } files
}
selected-multi = { $count ->
    [one] 1 item ({ $size })
   *[other] { $count } items ({ $size })
}
exact-bytes = ({ $bytes } bytes)
age-now = now
age-minutes = { $m }m ago
age-hours = { $h }h ago
age-days = { $d }d ago
ts-today = Today, { $time }
ts-yesterday = Yesterday, { $time }

# Preferences Tabs & About
prefs-tab-general = General
about-tab = About
about-subtitle = Native high-performance file manager for Linux & Windows
about-version-label = Version
about-developer-label = Developer
about-developer-value = in development
about-stage-label = Stage
about-stage-alpha = Alpha — Development Preview
about-license-label = License
about-stack-label = Engine
about-copy-info = Copy system info
about-info-copied = System info copied

# Folder properties (contents & disk space)
prop-contents-label = Contents
prop-free-space-label = Free space
hidden-items-count = { $count ->
    [one] 1 hidden
   *[other] { $count } hidden
}

# Tooltips
tip-new-tab = Open new tab (Ctrl+T)
tip-close-tab = Close tab (Ctrl+W)
tip-language = Change interface language
tip-theme = Toggle light/dark theme
tip-back = Go back (Alt+←)
tip-forward = Go forward (Alt+→)
tip-up = Go to parent folder (Alt+↑)
tip-home = Go to Home folder
tip-edit-path = Edit path directly (Ctrl+L)
tip-deep-search = Search recursively in subfolders (Ctrl+Shift+F)
tip-view-grid = Grid view
tip-view-list = Detail Table view
tip-view-columns = Miller Columns view
tip-new-item = Create new folder or file
tip-group-folders = Group folders before files in Grid
tip-zoom-out = Decrease card size (Ctrl+-)
tip-zoom-reset = Reset default card size (Ctrl+0)
tip-zoom-in = Increase card size (Ctrl++)
tip-sort = Cycle sort field (Name, Size, Type, Modified)
tip-sort-dir = Toggle ascending or descending order
tip-filter = Filter items by folder or extension
tip-hidden = Show or hide hidden files (Ctrl+H)
tip-terminal = Open terminal in this folder (Ctrl+Shift+T)
tip-favorite = Pin or unpin folder from favorites
tip-inspector = Show or hide Inspector panel (Alt+I)
tip-preferences = Open Preferences (right-click for quick menu)
tip-toggle-sidebar = Collapse or expand sidebar (Ctrl+B)
