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
view-columns = Columnas
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
tip-view-columns = Vista en Columnas (Miller)
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
