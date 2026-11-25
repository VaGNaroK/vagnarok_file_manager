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

# User places & drive mounting
music = Music
videos = Videos
public-dir = Public
templates = Templates
filesystem-root = File System
mount-drive = Mount
unmount-drive = Unmount
unmounted-label = Unmounted
refresh-drives = Refresh drives
tip-expand-home = Expand or collapse Home folders
open-as-root = Open as root
root-mode-badge = Root Mode (Elevated privileges)
grid-center-label = Grid item alignment
grid-align-left = Left to right
grid-align-center = Centered
tip-grid-alignment = Toggle Grid alignment (Left to right or Centered)
tip-system-icons = Toggle between default icons and host system icons
perm-allow-exec = Enable execution as program
open-with = Open with...
prop-tab-open-with = Open With
open-with-prompt = Select an application to open this file and other files of the same type:
open-with-default-app = Default application
open-with-recommended-apps = Recommended applications
open-with-other-apps = Other applications
open-with-custom-hint = Or enter/select a custom executable or command:
open-with-custom-placeholder = e.g. mpv --no-terminal or /usr/bin/program
open-with-add-to-list = Add to list
open-with-set-default = Set as default
open-with-reset-default = Reset to system defaults
open-with-open-once = Open now
open-with-loading = Discovering installed applications...
open-with-empty = No compatible applications found.
open-with-saved = Default application updated.
open-with-reset-done = System default restored.
open-with-pick-exec = Browse executable file

# Bulk Rename
br-title = Bulk Rename
br-menu-item = Bulk Rename...
br-clear-rules = Clear rules
br-rules-header = What do you want to do?
br-preview-header = Live Preview
br-col-status = Status
br-col-original = Current Name
br-col-new = New Name
br-unchanged = (no change)
br-skipped-folder = (folder skipped)
br-conflict-internal = Conflict: duplicate target name
br-conflict-external = Conflict: already exists on disk
br-invalid-name = Invalid filename
br-replace-title = Find & Replace
br-find-label = Find
br-replace-label = Replace with
br-where-label = Where
br-scope-anywhere = Anywhere
br-scope-start-name = Start of name only
br-scope-end-name = End of name only
br-scope-word-start = Start of word
br-scope-word-end = End of word
br-scope-whole-word = Whole word
br-match-case = Match UPPER/lower case
br-first-only = First match only
br-advanced-regex = Advanced options (Regex)
br-use-regex = Use regular expression (Regex)
br-remove-title = Remove Text / Characters
br-remove-first = First chars
br-remove-last = Last chars
br-remove-text-label = Remove text
br-remove-digits = Numbers (0-9)
br-remove-symbols = Symbols
br-collapse-spaces = Extra spaces
br-insert-title = Insert Text
br-insert-text-label = Text to insert
br-insert-pos-label = Position
br-insert-start = At start of name
br-insert-end = At end of name
br-insert-offset-start = At position (from start)
br-insert-offset-end = At position (from end)
br-case-title = Upper / Lowercase
br-case-keep = No change
br-case-upper = ALL UPPERCASE
br-case-lower = all lowercase
br-case-title-words = First Letter Of Each Word
br-case-sentence = Only 1st letter capitalized
br-number-title = Sequential Numbering
br-number-append = At end of name
br-number-prepend = At start of name
br-number-replace = Replace whole name
br-number-start = Start
br-number-step = Step
br-number-digits = Digits
br-number-sep = Separator
br-sort-label = Order
br-sort-selection = Selection order
br-sort-name-asc = Name (A-Z)
br-sort-name-desc = Name (Z-A)
br-sort-date-desc = Newest first
br-sort-date-asc = Oldest first
br-sort-size-asc = Smallest first
br-sort-size-desc = Largest first
br-media-title = Photo / Music Info (EXIF & ID3)
br-media-exif-ymd = Photo date (YYYY-MM-DD)
br-media-exif-datetime = Photo date & time
br-media-exif-camera = Camera model
br-media-id3-track = Track #
br-media-id3-artist = Artist
br-media-id3-album = Album
br-media-id3-title = Song title
br-media-id3-year = Year
br-media-loading = Reading EXIF/ID3 tags...
br-ext-title = File Extension
br-ext-keep = Keep original
br-ext-lower = lowercase (.jpg)
br-ext-upper = UPPERCASE (.JPG)
br-ext-replace = Change to
br-ext-remove = Remove extension
br-include-folders = Include folders
br-apply = Apply
br-applying = Renaming...
br-undo-last = Undo last bulk rename
br-readonly-error = Bulk rename is not supported in Trash or compressed archives.

# Archive extraction & compression
arch-extract-here = Extract here
arch-extract-subdir = Extract to subfolder
arch-compress-as = Compress as...
arch-add-7z = Add to .7z archive
arch-modal-title = Compress as...
arch-output-name = Archive name
arch-destination = Destination
arch-format-label = Archive format
arch-level-label = Compression level
arch-format-7z = .7z (7-Zip — High compression)
arch-format-zip = .zip (ZIP — Universal)
arch-format-tar = .tar (TAR — Uncompressed bundle)
arch-format-targz = .tar.gz (TAR.GZ — Gzip compressed)
arch-level-store = Store (no compression)
arch-level-min = Minimum (fast)
arch-level-normal = Normal (balanced)
arch-level-max = Maximum (best compression)
arch-tar-store-hint = TAR bundles files without compression (Store only).
arch-conflict-label = If file exists
arch-conflict-rename = Auto-rename
arch-conflict-overwrite = Overwrite
arch-conflict-skip = Cancel if exists
arch-start-compress = Compress
arch-minimize = Minimize
arch-restore = Restore
arch-compressing = Compressing
arch-extracting = Extracting
arch-done-compress = Archive created
arch-done-extract = Extraction completed
arch-cancelled = Operation cancelled
arch-password-title = Password-protected archive
arch-password-hint = Enter the password to extract this archive.
arch-password-placeholder = Password...
arch-password-wrong = Incorrect password. Please try again.
arch-password-submit = Extract
arch-password-toggle = Show or hide password

# Duplicate File Finder
dup-menu-action = Find duplicate files...
dup-modal-title = Find Duplicate Files
dup-readonly-error = Duplicate finder is not supported in Trash or compressed archives.
dup-confirm-level-label = Duplicate confirmation level
dup-level-quick = 1. Fast — Size + Partial MD5 (Triage)
dup-level-standard = 2. Standard — Size + Partial MD5 + Full MD5
dup-level-sha256 = 3. Safe — Full MD5 + SHA-256 Confirmation
dup-level-byte = 4. Maximum — Full MD5 + Byte-by-Byte Comparison
dup-min-size-label = Minimum size
dup-min-size-any = Any (>= 1 B)
dup-min-size-1kb = >= 1 KB
dup-min-size-1mb = >= 1 MB
dup-min-size-10mb = >= 10 MB
dup-start-scan = Start Scan
dup-rescan = Rescan
dup-stop-scan = Stop
dup-opt-subfolders = Include subfolders
dup-opt-hidden = Include hidden
dup-opt-ignore-empty = Ignore empty (0 B)
dup-opt-ignore-hardlinks = Ignore hardlinks
dup-quick-warning = Fast mode only compares start, middle and end chunks. For permanent deletion, Standard, Safe or Maximum is recommended.
dup-quick-select-label = Quick select:
dup-keep-oldest = Keep oldest
dup-keep-newest = Keep newest
dup-keep-shortest = Shortest path
dup-clear-selection = Clear
dup-badge-oldest = Oldest
dup-badge-hardlink = Hardlink
dup-verify-partial = Partial MD5 (approx.)
dup-verify-md5 = Full MD5 ✓
dup-verify-sha256 = MD5 + SHA-256 ✓
dup-verify-byte = Byte-by-Byte 100% ✓
dup-stage-walking = Scanning folder
dup-stage-partial = Partial MD5
dup-stage-full = Full MD5
dup-stage-sha256 = MD5 + SHA-256
dup-stage-byte = MD5 + Byte-by-Byte
dup-all-selected-warning = Warning: ALL copies of at least 1 group are selected! Uncheck 1 copy so you do not lose the original file.
dup-delete-permanent = Delete permanently
dup-move-to-trash = Move to Trash
dup-confirm-perm-delete = Confirm permanent delete?
dup-empty-idle = Choose a confirmation level and click Start Scan to find duplicate files.
dup-empty-none = No duplicate files found with the current filters.
dup-group-prefix = Group
dup-copies-word = copies
dup-each-word = each
dup-selected-word = selected
dup-groups-found = groups found
dup-reclaimable = reclaimable
dup-footer-prefix = Duplicates
dup-cancelled = Duplicate scan cancelled
dup-stale-skipped = Some files changed or disappeared on disk and were skipped for safety.
dup-last-copy-protected = Deletion blocked in group(s) whose last unselected copy no longer exists on disk.
dup-delete-done = Duplicate files removed safely
ctx-send-to = Send to...
ctx-shortcut-desktop = Create Desktop shortcut
ctx-shortcut-here-unix = Create symbolic link here
ctx-shortcut-here-win = Create shortcut here
shortcut-desktop-missing = Desktop folder not found.
filter-ext-placeholder = e.g. .exe, .zip...
shortcuts-created =
    { $count ->
        [one] Shortcut created.
       *[other] { $count } shortcuts created.
    }
undo-action = Undo (Ctrl+Z)
undo-trash-action = Undo delete (Ctrl+Z)
moved-to-trash-undo-hint = Moved to Trash (Ctrl+Z to undo)
undo-nothing-in-trash = Item no longer in Trash to restore.
undo-rename-done = Rename undone.
undo-restored-count =
    { $count ->
        [one] Restored from Trash.
       *[other] Restored { $count } items from Trash.
    }
mount-disk-image = Mount disk image
unmount-disk-image = Unmount image
eject-safely = Safely remove
image-mounted = Disk image mounted
drive-ejected = Drive safely removed
drive-free-label = free
drive-click-to-mount = Click to mount
tip-split-view = Split view (F3)
close-split-pane = Close split pane (F3)
drag-move-prefix = Move
drag-copy-prefix = Copy
drag-ctrl-copy-hint = Ctrl = Copy
update-available-banner = New stable version available:
update-download-btn = Download update
update-ignore-btn = Ignore this version
update-check-btn = Check for updates
update-checking = Checking for updates...
update-up-to-date = You are already on the latest stable version.
update-check-failed = Could not check for updates right now.
update-auto-check-label = Check for stable updates on startup
update-open-releases = Open release page
tip-update-available = New stable version available — click to open the release page
recent-files = Recent
prefs-recent-files-label = Show recent files
clear-recent-files = Clear recent
remove-from-recent = Remove from recent
open-file-location = Open file location
open-target-location = Open target location
prefs-startup-location-label = Startup folder
startup-home = Home folder
startup-last-visited = Last visited folder
ntfs-object-name = Object name:
ntfs-principals-label = Group or user names:
ntfs-edit-hint = To change permissions, click Edit.
ntfs-edit-active-hint = Toggle Allow or Deny individually (or use Full control).
ntfs-edit-btn = Edit...
ntfs-done-edit-btn = Done editing
ntfs-add-btn = Add...
ntfs-remove-btn = Remove
ntfs-permissions-for = Permissions for
ntfs-allow-col = Allow
ntfs-deny-col = Deny
ntfs-full-control = Full control
ntfs-list-folder = List folder contents
ntfs-delete-modify = Delete / Modify
ntfs-inherited-badge = Inherited
ntfs-explicit-badge = Explicit
ntfs-owner-label = Owner
ntfs-inheritance-label = NTFS Inheritance
ntfs-inheritance-on = Enabled (inheriting from parent)
ntfs-inheritance-off = Disabled (explicit)
ntfs-add-placeholder = e.g. Users, Everyone, Administrators...
ntfs-loading = Loading NTFS permissions...
posix-owner-group-header = Owner & Group (UID / GID)
posix-octal-symbolic-header = Octal & Symbolic Mode
posix-presets-label = Presets:
posix-preset-default = Default
posix-preset-exec = Executable
posix-preset-private = Private
posix-preset-readonly = Read-only
posix-preset-group = Group
posix-preset-shared = Shared
posix-scope-col = Scope
posix-summary-col = Summary
posix-others-sub = all other users
posix-special-bits-header = Special POSIX Bits
posix-quick-action-header = Quick Action
posix-recursive-header = Subfolder Scope
posix-apply-recursive = Apply recursively to contents

analyze-space = Analyze space
tip-analyze-space = Analyze disk space usage and large files
disk-scan-eyebrow = Analyzing
disk-scan-title = Scanning for large files...
disk-scan-current-item = Current item
disk-scan-files-checked = Files checked
disk-scan-data-checked = Data checked
disk-scan-elapsed = Elapsed time
disk-scan-hint = Scanning the selected location; duration depends on the number of files
disk-results-title = Disk Space Analysis
disk-root-crumb = Selected root
disk-treemap-title = Proportional map (click to explore)
disk-top-consumers = Top consumers at this level
disk-by-type = By file type
disk-mark-cleanup = Select files or folders to delete
disk-reanalyze = Rescan
disk-clear-selection = Clear selection
disk-reclaim-suffix = will be freed
disk-mode-files = Largest files
disk-mode-folders = Folders
disk-cat-iso = ISOs / Disks
disk-cat-build = Builds / Cache
disk-cat-archives = Archives
disk-cat-media = Media
disk-cat-docs = Documents
disk-cat-other = Other

search-name-label = Name / Path
search-containing-label = Search containing...
search-containing-placeholder = Text or regex inside files (skips binaries)...
search-size-label = Size
search-date-label = Modified
search-type-label = Type
search-hidden-label = Hidden
search-streaming-hint = Index-free streaming • Skips binaries
search-any = Any
search-today = Today
search-last-7d = 7 days
search-last-30d = 30 days
search-type-code = Code
search-type-docs = Documents
search-type-images = Images
search-type-audio = Audio
search-type-video = Video
search-type-archives = Archives
