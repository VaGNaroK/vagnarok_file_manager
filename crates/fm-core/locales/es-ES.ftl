# Español (ES) — Locale para Vagnarok File Manager

# Barra lateral y navegación
quick-access = ACCESO RÁPIDO
drives = UNIDADES
tags-and-system = ETIQUETAS Y SISTEMA
documents = Documentos
home = Inicio
desktop = Escritorio
downloads = Descargas
pictures = Imágenes
code-dev = Código / Dev
starred = Favoritos
trash = Papelera
preferences = Preferencias

# Encabezado y búsqueda
search-placeholder = Buscar en este directorio...
deep-search = Subcarpetas
deep-search-title = Búsqueda Profunda (Fuzzy)
deep-search-empty = No se encontraron archivos en las subcarpetas.
searching = Buscando en subcarpetas...
no-active-tab = Ninguna pestaña activa
follow-system = Sistema (Auto)

# Barra de acciones, lienzo y vistas
new-item = + Nuevo
sort = Ordenar
hidden = Ocultos
folders = CARPETAS
files = ARCHIVOS
total-size = TAMAÑO DIRECTO
open-terminal = Terminal
pin-favorite = Fijar
unpin-favorite = Desfijar
remove-favorite = Quitar de favoritos
filter-all = Todos
filter-ext = Filtrar
directories-only = Carpetas
archive-readonly = Archivo comprimido (solo lectura)
grid-group-label = Agrupar carpetas en Cuadrícula
dirs-first-label = Carpetas primero al ordenar
recursive-size-label = TAMAÑO TOTAL
empty-trash = Vaciar papelera
restore-item = Restaurar
confirm-empty-trash-msg = ¿Vaciar permanentemente todos los elementos de la papelera del sistema?
trash-emptied = Papelera vaciada.
trash-restored = Elemento(s) restaurado(s) a su ubicación original.
empty-subdir = Subcarpeta vacía
loading = Cargando directorio...
empty-dir = Carpeta vacía
go-back = Volver
retry = Reintentar
root-fs = Sistema
dir-badge = DIR
file-badge = ARCH

# Columnas de tabla de detalles
col-name = Nombre
col-size = Tamaño
col-type = Tipo
col-perms = Permisos
col-modified = Modificado

# Inspector y QuickLook
inspector-title = Inspector
inspector-empty = Selecciona un elemento para ver metadatos, permisos y checksum.
permissions = Permisos
modified = Modificado
created = Creado
checksum = SHA-256
mime-type = Tipo MIME
computing-hash = calculando...
quicklook-hint = Space: QuickLook
quicklook-title = QuickLook
close = Cerrar
binary-preview-hint = Archivo binario o demasiado grande — sin vista previa directa.
git-status-label = Estado Git

# Pie y transferencias
selected-none = Seleccionado: Ninguno
free-of = libres de
clipboard-copy-tag = Copiados
clipboard-cut-tag = Cortados
transferring = Transfiriendo...

# Operaciones de archivo y modales
new-folder = Nueva Carpeta
new-file = Nuevo Archivo
rename = Renombrar (F2)
copy = Copiar (Ctrl+C)
cut = Cortar (Ctrl+X)
paste = Pegar (Ctrl+V)
move-to-trash = Enviar a la Papelera (Del)
delete-permanent = Eliminar Permanentemente (Shift+Del)
confirm = Confirmar
cancel = Cancelar
create = Crear
name-label = Nombre
invalid-name = Nombre no válido.
confirm-trash-msg = ¿Enviar los elementos seleccionados a la papelera del sistema?
confirm-delete-msg = Esta acción elimina permanentemente del disco y no se puede deshacer.
conflict-title = Conflicto en Destino
conflict-msg = Algunos elementos ya existen en el destino. ¿Cómo deseas continuar?
conflict-rename = Conservar ambos (Renombrar)
conflict-overwrite = Sobrescribir
conflict-skip = Omitir existentes
open-item = Abrir
open-in-new-tab = Abrir en nueva pestaña
copy-path = Copiar ruta
select-all = Seleccionar todo (Ctrl+A)
path-copied = Ruta copiada
properties = Propiedades

# Preferencias
prefs-title = Preferencias
theme-label = Tema
lang-label = Idioma
hero-card-label = Mostrar tarjeta resumen de carpeta
decorations-label = Barra de ventana
default-view-label = Vista predeterminada
view-grid = Cuadrícula
view-list = Lista
window-bar-native = Barra Nativa
window-bar-csd = CSD Integrado

# Ventana de Propiedades
location-label = Ubicación
copy-sha256 = Copiar SHA-256
hash-copied = SHA-256 copiado
properties-header-prefix = { "Propiedades: " }
prop-tab-general = General
prop-tab-security = Seguridad
prop-tab-history = Historial
prop-tab-metadata = Metadatos
prop-tab-permissions = Permisos
prop-kind-label = Tipo
prop-system-mime-label = Sistema / MIME
prop-accessed-label = Último acceso
prop-quick-props = Propiedades Rápidas
prop-readonly = Solo lectura
prop-hidden = Oculto
prop-advanced-settings = Configuración Avanzada
prop-confirm = Confirmar
prop-apply = Aplicar
prop-open-default = Abrir predeterminado
prop-owner = Propietario
prop-group = Grupo
prop-others = Otros
prop-octal-mode = Modo octal
prop-symlink-target = Destino del enlace
prop-archive-notice = Solo lectura (dentro de archivo comprimido)
prop-applied-notice = Propiedades aplicadas.
prop-pending-changes = Cambios pendientes
prop-calculating = Calculando...
prop-sha-na-folders = No aplicable a carpetas
prop-sha-computing = Calculando hash SHA-256...
yes-label = Sí
no-label = No
ext-label = Extensión
perm-read = Lectura
perm-write = Escritura
perm-execute = Ejecución

# Tipos de entrada
kind-folder = Carpeta de Archivos
kind-symlink-dir = Enlace simbólico (Carpeta)
kind-symlink-file = Enlace simbólico (Archivo)
kind-special = Elemento Especial
kind-file = Archivo
kind-file-ext = Archivo (.{ $ext })

# Formato pluralizado y parametrizado
items-count = { $count ->
    [one] 1 elemento
   *[other] { $count } elementos
}
folders-count = { $count ->
    [one] 1 carpeta
   *[other] { $count } carpetas
}
files-count = { $count ->
    [one] 1 archivo
   *[other] { $count } archivos
}
selected-multi = { $count ->
    [one] 1 elemento ({ $size })
   *[other] { $count } elementos ({ $size })
}
exact-bytes = ({ $bytes } bytes)
age-now = ahora
age-minutes = hace { $m }m
age-hours = hace { $h }h
age-days = hace { $d }d
ts-today = Hoy, { $time }
ts-yesterday = Ayer, { $time }

# Pestañas de Preferencias y Acerca de
prefs-tab-general = General
about-tab = Acerca de
about-subtitle = Gestor de archivos nativo de alto rendimiento para Linux y Windows
about-version-label = Versión
about-developer-label = Desarrollador
about-developer-value = en desarrollo
about-stage-label = Etapa
about-stage-alpha = Alpha — Vista previa de desarrollo
about-license-label = Licencia
about-stack-label = Motor / Stack
about-copy-info = Copiar información
about-info-copied = Información copiada

# Propiedades de carpeta (contenido y espacio en disco)
prop-contents-label = Contenido
prop-free-space-label = Espacio libre
hidden-items-count = { $count ->
    [one] 1 oculto
   *[other] { $count } ocultos
}

# Sugerencias (Tooltips)
tip-new-tab = Abrir nueva pestaña (Ctrl+T)
tip-close-tab = Cerrar pestaña (Ctrl+W)
tip-language = Cambiar idioma de la interfaz
tip-theme = Alternar tema claro/oscuro
tip-back = Ir atrás (Alt+←)
tip-forward = Ir adelante (Alt+→)
tip-up = Subir a la carpeta superior (Alt+↑)
tip-home = Ir a la carpeta de Inicio
tip-edit-path = Editar ruta directamente (Ctrl+L)
tip-deep-search = Buscar recursivamente en subcarpetas (Ctrl+Shift+F)
tip-view-grid = Vista en Cuadrícula
tip-view-list = Vista en Tabla de Detalles
tip-new-item = Crear nueva carpeta o archivo
tip-group-folders = Agrupar carpetas antes que archivos
tip-zoom-out = Reducir tamaño de tarjetas (Ctrl+-)
tip-zoom-reset = Restablecer tamaño predeterminado (Ctrl+0)
tip-zoom-in = Aumentar tamaño de tarjetas (Ctrl++)
tip-sort = Cambiar criterio de orden (Nombre, Tamaño, Tipo, Modificado)
tip-sort-dir = Cambiar orden ascendente o descendente
tip-filter = Filtrar elementos por carpeta o extensión
tip-hidden = Mostrar u ocultar archivos ocultos (Ctrl+H)
tip-terminal = Abrir terminal en esta carpeta (Ctrl+Shift+T)
tip-favorite = Fijar o quitar carpeta de favoritos
tip-inspector = Mostrar u ocultar panel Inspector (Alt+I)
tip-preferences = Abrir Preferencias (clic derecho para menú rápido)
tip-toggle-sidebar = Contraer o expandir barra lateral (Ctrl+B)

# Lugares de usuario y montaje de discos
music = Música
videos = Vídeos
public-dir = Público
templates = Plantillas
filesystem-root = Sistema de archivos
mount-drive = Montar
unmount-drive = Desmontar
unmounted-label = Desmontado
refresh-drives = Actualizar unidades
tip-expand-home = Expandir o contraer carpetas de Inicio
open-as-root = Abrir como root
root-mode-badge = Modo Root (Privilegios elevados)
grid-center-label = Alineación de elementos en Cuadrícula
grid-align-left = De izquierda a derecha
grid-align-center = Centrado
tip-grid-alignment = Alternar alineación de cuadrícula (Izquierda a derecha o Centrado)
tip-system-icons = Alternar entre iconos predeterminados e iconos del sistema
perm-allow-exec = Habilitar ejecución como programa
open-with = Abrir con...
prop-tab-open-with = Abrir con
open-with-prompt = Selecciona una aplicación para abrir este archivo y otros del mismo tipo:
open-with-default-app = Aplicación predeterminada
open-with-recommended-apps = Aplicaciones recomendadas
open-with-other-apps = Otras aplicaciones
open-with-custom-hint = También puedes escribir o elegir un comando o ejecutable personalizado:
open-with-custom-placeholder = Ej.: mpv --no-terminal o /ruta/programa
open-with-add-to-list = Añadir a la lista
open-with-set-default = Establecer como predeterminada
open-with-reset-default = Restablecer valores del sistema
open-with-open-once = Abrir ahora
open-with-loading = Buscando aplicaciones instaladas...
open-with-empty = No se encontraron aplicaciones compatibles.
open-with-saved = Aplicación predeterminada actualizada.
open-with-reset-done = Predeterminado del sistema restaurado.
open-with-pick-exec = Seleccionar archivo ejecutable

# Renombrado por Lotes
br-title = Renombrar por Lotes
br-menu-item = Renombrar por Lotes...
br-clear-rules = Limpiar reglas
br-rules-header = ¿Qué quieres hacer?
br-preview-header = Vista previa en tiempo real
br-col-status = Estado
br-col-original = Nombre actual
br-col-new = Nombre nuevo
br-unchanged = (sin cambios)
br-skipped-folder = (carpeta omitida)
br-conflict-internal = Conflicto: nombre duplicado
br-conflict-external = Conflicto: ya existe en el disco
br-invalid-name = Nombre no válido
br-replace-title = Buscar y Reemplazar
br-find-label = Buscar
br-replace-label = Reemplazar por
br-where-label = Dónde aplicar
br-scope-anywhere = En cualquier lugar
br-scope-start-name = Solo al inicio del nombre
br-scope-end-name = Solo al final del nombre
br-scope-word-start = Inicio de palabra
br-scope-word-end = Fin de palabra
br-scope-whole-word = Palabra completa
br-match-case = Distinguir MAYÚS./minús.
br-first-only = Solo 1.ª coincidencia
br-advanced-regex = Opciones avanzadas (Regex)
br-use-regex = Usar expresión regular (Regex)
br-remove-title = Quitar Texto / Caracteres
br-remove-first = Primeros car.
br-remove-last = Últimos car.
br-remove-text-label = Quitar texto
br-remove-digits = Números (0-9)
br-remove-symbols = Símbolos
br-collapse-spaces = Espacios extra
br-insert-title = Insertar Texto
br-insert-text-label = Texto a insertar
br-insert-pos-label = Posición
br-insert-start = Al inicio del nombre
br-insert-end = Al final del nombre
br-insert-offset-start = En posición (desde inicio)
br-insert-offset-end = En posición (desde final)
br-case-title = Mayúsculas / Minúsculas
br-case-keep = Sin cambios
br-case-upper = TODO MAYÚSCULAS
br-case-lower = todo minúsculas
br-case-title-words = Primera Letra De Cada Palabra
br-case-sentence = Solo 1.ª letra en mayúscula
br-number-title = Numeración Secuencial
br-number-append = Al final del nombre
br-number-prepend = Al inicio del nombre
br-number-replace = Reemplazar todo el nombre
br-number-start = Inicio
br-number-step = Paso
br-number-digits = Dígitos
br-number-sep = Separador
br-sort-label = Orden
br-sort-selection = Orden seleccionado
br-sort-name-asc = Nombre (A-Z)
br-sort-name-desc = Nombre (Z-A)
br-sort-date-desc = Más recientes
br-sort-date-asc = Más antiguos
br-sort-size-asc = Menor tamaño
br-sort-size-desc = Mayor tamaño
br-media-title = Datos de Foto / Música (EXIF e ID3)
br-media-exif-ymd = Fecha de foto (AAAA-MM-DD)
br-media-exif-datetime = Fecha y hora de foto
br-media-exif-camera = Modelo de cámara
br-media-id3-track = N.º de pista
br-media-id3-artist = Artista
br-media-id3-album = Álbum
br-media-id3-title = Título de canción
br-media-id3-year = Año
br-media-loading = Leyendo etiquetas EXIF/ID3...
br-ext-title = Cambiar Extensión
br-ext-keep = Mantener original
br-ext-lower = minúsculas (.jpg)
br-ext-upper = MAYÚSCULAS (.JPG)
br-ext-replace = Cambiar a
br-ext-remove = Quitar extensión
br-include-folders = Incluir carpetas
br-apply = Aplicar
br-applying = Renombrando...
br-undo-last = Deshacer último lote
br-readonly-error = El renombrado por lotes no está disponible en la Papelera ni en archivos comprimidos.

# Extracción y compresión de archivos
arch-extract-here = Extraer aquí
arch-extract-subdir = Extraer en subcarpeta
arch-compress-as = Comprimir como...
arch-add-7z = Añadir a archivo .7z
arch-modal-title = Comprimir como...
arch-output-name = Nombre del archivo
arch-destination = Carpeta de destino
arch-format-label = Formato / Extensión
arch-level-label = Nivel de compresión
arch-format-7z = .7z (7-Zip — Alta compresión)
arch-format-zip = .zip (ZIP — Universal)
arch-format-tar = .tar (TAR — Sin compresión)
arch-format-targz = .tar.gz (TAR.GZ — Comprimido con Gzip)
arch-level-store = Almacenar (sin compresión)
arch-level-min = Mínimo (rápido)
arch-level-normal = Normal (equilibrado)
arch-level-max = Máximo (mayor compresión)
arch-tar-store-hint = TAR empaqueta archivos sin compresión (solo Almacenar).
arch-conflict-label = Si ya existe
arch-conflict-rename = Renombrar automáticamente
arch-conflict-overwrite = Sobrescribir
arch-conflict-skip = Cancelar si existe
arch-start-compress = Comprimir
arch-minimize = Minimizar
arch-restore = Restaurar
arch-compressing = Comprimiendo
arch-extracting = Extrayendo
arch-done-compress = Archivo creado
arch-done-extract = Extracción completada
arch-cancelled = Operación cancelada
arch-password-title = Archivo protegido con contraseña
arch-password-hint = Introduce la contraseña para extraer este archivo.
arch-password-placeholder = Contraseña...
arch-password-wrong = Contraseña incorrecta. Inténtalo de nuevo.
arch-password-submit = Extraer
arch-password-toggle = Mostrar u ocultar contraseña

# Buscador de archivos duplicados
dup-menu-action = Buscar archivos duplicados...
dup-modal-title = Buscar Archivos Duplicados
dup-readonly-error = La búsqueda de duplicados no es compatible con la Papelera ni con archivos comprimidos.
dup-confirm-level-label = Nivel de confirmación de duplicidad
dup-level-quick = 1. Rápido — Tamaño + MD5 Parcial (Triaje)
dup-level-standard = 2. Estándar — Tamaño + MD5 Parcial + MD5 Completo
dup-level-sha256 = 3. Seguro — MD5 Completo + Confirmación SHA-256
dup-level-byte = 4. Máximo — MD5 Completo + Comparación Byte a Byte
dup-min-size-label = Tamaño mínimo
dup-min-size-any = Cualquiera (≥ 1 B)
dup-min-size-1kb = ≥ 1 KB
dup-min-size-1mb = ≥ 1 MB
dup-min-size-10mb = ≥ 10 MB
dup-start-scan = Iniciar búsqueda
dup-rescan = Re-escanear
dup-stop-scan = Detener
dup-opt-subfolders = Incluir subcarpetas
dup-opt-hidden = Incluir ocultos
dup-opt-ignore-empty = Ignorar vacíos (0 B)
dup-opt-ignore-hardlinks = Ignorar hardlinks
dup-quick-warning = El modo Rápido solo compara inicio, medio y fin de los archivos. Para borrado permanente se recomienda Estándar, Seguro o Máximo.
dup-quick-select-label = Selección rápida:
dup-keep-oldest = Conservar más antiguo
dup-keep-newest = Conservar más reciente
dup-keep-shortest = Ruta más corta
dup-clear-selection = Limpiar
dup-badge-oldest = Más antiguo
dup-badge-hardlink = Hardlink
dup-verify-partial = MD5 Parcial (aprox.)
dup-verify-md5 = MD5 Completo ✓
dup-verify-sha256 = MD5 + SHA-256 ✓
dup-verify-byte = Byte a Byte 100% ✓
dup-stage-walking = Escaneando carpeta
dup-stage-partial = MD5 Parcial
dup-stage-full = MD5 Completo
dup-stage-sha256 = MD5 + SHA-256
dup-stage-byte = MD5 + Byte a Byte
dup-all-selected-warning = ¡Atención: marcaste TODAS las copias de al menos 1 grupo! Desmarca 1 copia para no perder el archivo original.
dup-delete-permanent = Eliminar permanentemente
dup-move-to-trash = Mover a la Papelera
dup-confirm-perm-delete = ¿Confirmar borrado permanente?
dup-empty-idle = Elige el nivel de confirmación y haz clic en Iniciar búsqueda para encontrar duplicados.
dup-empty-none = No se encontraron archivos duplicados con los filtros actuales.
dup-group-prefix = Grupo
dup-copies-word = copias
dup-each-word = cada uno
dup-selected-word = seleccionados
dup-groups-found = grupos encontrados
dup-reclaimable = recuperables
dup-footer-prefix = Duplicados
dup-cancelled = Búsqueda de duplicados cancelada
dup-stale-skipped = Algunos archivos cambiaron o desaparecieron del disco y se omitieron por seguridad.
dup-last-copy-protected = Eliminación bloqueada en grupo(s) cuya última copia sin marcar ya no existe en disco.
dup-delete-done = Duplicados eliminados con seguridad
ctx-send-to = Enviar a...
ctx-shortcut-desktop = Crear acceso directo en el Escritorio
ctx-shortcut-here-unix = Crear enlace simbólico aquí
ctx-shortcut-here-win = Crear acceso directo aquí
shortcut-desktop-missing = No se encontró la carpeta Escritorio.
filter-ext-placeholder = ej.: .exe, .zip...
shortcuts-created =
    { $count ->
        [one] Acceso directo creado correctamente.
       *[other] { $count } accesos directos creados correctamente.
    }
undo-action = Deshacer (Ctrl+Z)
undo-trash-action = Deshacer eliminación (Ctrl+Z)
moved-to-trash-undo-hint = Movido a la Papelera (Ctrl+Z para deshacer)
undo-nothing-in-trash = Elemento no encontrado en la Papelera para restaurar.
undo-rename-done = Cambio de nombre deshecho.
undo-restored-count =
    { $count ->
        [one] Restaurado desde la Papelera.
       *[other] { $count } elementos restaurados desde la Papelera.
    }
mount-disk-image = Montar imagen de disco
unmount-disk-image = Desmontar imagen
eject-safely = Expulsar con seguridad
image-mounted = Imagen montada correctamente
drive-ejected = Unidad expulsada con seguridad
drive-free-label = libres
drive-click-to-mount = Haz clic para montar
tip-split-view = Dividir pantalla (F3)
close-split-pane = Cerrar panel dividido (F3)
drag-move-prefix = Mover
drag-copy-prefix = Copiar
drag-ctrl-copy-hint = Ctrl = Copiar
update-available-banner = Nueva versión estable disponible:
update-download-btn = Descargar actualización
update-ignore-btn = Ignorar esta versión
update-check-btn = Buscar actualizaciones
update-checking = Buscando actualizaciones...
update-up-to-date = Ya tienes la versión estable más reciente.
update-check-failed = No se pudieron buscar actualizaciones ahora.
update-auto-check-label = Buscar actualizaciones estables al iniciar
update-open-releases = Abrir página de la versión
tip-update-available = Nueva versión estable disponible — haz clic para abrir la página de descarga
recent-files = Recientes
prefs-recent-files-label = Mostrar archivos recientes
clear-recent-files = Limpiar recientes
remove-from-recent = Quitar de recientes
open-file-location = Abrir ubicación del archivo
open-target-location = Abrir ubicación de destino
prefs-startup-location-label = Carpeta inicial al abrir
startup-home = Carpeta de usuario
startup-last-visited = Última carpeta abierta
ntfs-object-name = Nombre del objeto:
ntfs-principals-label = Nombres de grupos o usuarios:
ntfs-edit-hint = Para cambiar permisos, haz clic en Editar.
ntfs-edit-active-hint = Marca Permitir o Denegar individualmente (o usa Control total).
ntfs-edit-btn = Editar...
ntfs-done-edit-btn = Finalizar edición
ntfs-add-btn = Agregar...
ntfs-remove-btn = Quitar
ntfs-permissions-for = Permisos de
ntfs-allow-col = Permitir
ntfs-deny-col = Denegar
ntfs-full-control = Control total
ntfs-list-folder = Mostrar contenido de la carpeta
ntfs-delete-modify = Eliminar / Modificar
ntfs-inherited-badge = Heredado
ntfs-explicit-badge = Explícito
ntfs-owner-label = Propietario
ntfs-inheritance-label = Herencia NTFS
ntfs-inheritance-on = Activada (heredando del padre)
ntfs-inheritance-off = Desactivada (explícitos)
ntfs-add-placeholder = Ej.: Usuarios, Todos, Administradores...
ntfs-loading = Cargando permisos NTFS...
posix-owner-group-header = Propietario y Grupo (UID / GID)
posix-octal-symbolic-header = Modo Octal y Simbólico
posix-presets-label = Preajustes:
posix-preset-default = Predeterminado
posix-preset-exec = Ejecutable
posix-preset-private = Privado
posix-preset-readonly = Solo lectura
posix-preset-group = Grupo
posix-preset-shared = Compartida
posix-scope-col = Ámbito
posix-summary-col = Resumen
posix-others-sub = todos los usuarios
posix-special-bits-header = Bits Especiales POSIX
posix-quick-action-header = Acceso Rápido
posix-recursive-header = Ámbito en Subcarpetas
posix-apply-recursive = Aplicar recursivamente al contenido

analyze-space = Analizar espacio
tip-analyze-space = Analizar uso de espacio en disco y archivos grandes
disk-scan-eyebrow = Analizando
disk-scan-title = Buscando archivos grandes...
disk-scan-current-item = Elemento actual
disk-scan-files-checked = Archivos verificados
disk-scan-data-checked = Datos verificados
disk-scan-elapsed = Tiempo transcurrido
disk-scan-hint = Analizando la ubicación seleccionada; el tiempo depende de la cantidad de archivos
disk-results-title = Análisis de Espacio en Disco
disk-root-crumb = Raíz seleccionada
disk-treemap-title = Mapa proporcional (clic para explorar)
disk-top-consumers = Mayores consumidores en este nivel
disk-by-type = Por tipo de archivo
disk-mark-cleanup = Marca archivos o carpetas para eliminar
disk-reanalyze = Reanalizar
disk-clear-selection = Limpiar selección
disk-reclaim-suffix = se liberarán
disk-mode-files = Archivos más grandes
disk-mode-folders = Carpetas
disk-cat-iso = ISOs / Discos
disk-cat-build = Builds / Caché
disk-cat-archives = Comprimidos
disk-cat-media = Multimedia
disk-cat-docs = Documentos
disk-cat-other = Otros

search-name-label = Nombre / Ruta
search-containing-label = Buscar contenido...
search-containing-placeholder = Texto o regex dentro de archivos (omite binarios)...
search-size-label = Tamaño
search-date-label = Modificado
search-type-label = Tipo
search-hidden-label = Ocultos
search-streaming-hint = Streaming sin índice • Omite binarios
search-any = Cualquiera
search-today = Hoy
search-last-7d = 7 días
search-last-30d = 30 días
search-type-code = Código
search-type-docs = Documentos
search-type-images = Imágenes
search-type-audio = Audio
search-type-video = Vídeo
search-type-archives = Archivos
