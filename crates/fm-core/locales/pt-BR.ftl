# Português (Brasil) — Locale para Vagnarok File Manager

# Barra lateral e navegação
quick-access = ACESSO RÁPIDO
drives = UNIDADES
tags-and-system = TAGS E SISTEMA
documents = Documentos
home = Início
desktop = Área de Trabalho
downloads = Downloads
pictures = Imagens
code-dev = Código / Dev
starred = Favoritos
trash = Lixeira
preferences = Preferências

# Cabeçalho e busca
search-placeholder = Buscar neste diretório...
deep-search = Subpastas
deep-search-title = Busca Profunda (Fuzzy)
deep-search-empty = Nenhum arquivo encontrado nas subpastas.
searching = Buscando em subpastas...
no-active-tab = Nenhuma aba ativa
follow-system = Sistema (Auto)

# Barra de ações, canvas e visualizações
new-item = + Novo
sort = Ordenar
hidden = Ocultos
folders = PASTAS
files = ARQUIVOS
total-size = TAMANHO DIRETO
open-terminal = Terminal
pin-favorite = Fixar
unpin-favorite = Desafixar
remove-favorite = Remover dos favoritos
filter-all = Todos
filter-ext = Filtrar
directories-only = Pastas
archive-readonly = Arquivo compactado (somente leitura)
grid-group-label = Agrupar pastas no Grid
dirs-first-label = Pastas primeiro na ordenação
recursive-size-label = TAMANHO TOTAL
empty-trash = Esvaziar lixeira
restore-item = Restaurar
confirm-empty-trash-msg = Deseja esvaziar permanentemente todos os itens da lixeira do sistema?
trash-emptied = Lixeira esvaziada.
trash-restored = Itens restaurados para o local original.
empty-subdir = Subpasta vazia
loading = Carregando diretório...
empty-dir = Pasta vazia
go-back = Voltar
retry = Tentar novamente
root-fs = Sistema
dir-badge = DIR
file-badge = ARQ

# Colunas da tabela de detalhes
col-name = Nome
col-size = Tamanho
col-type = Tipo
col-perms = Permissões
col-modified = Modificado

# Inspetor e QuickLook
inspector-title = Inspetor
inspector-empty = Selecione um item para ver metadados, permissões e checksum.
permissions = Permissões
modified = Modificado
created = Criado em
checksum = SHA-256
mime-type = Tipo MIME
computing-hash = calculando...
quicklook-hint = Space: QuickLook
quicklook-title = QuickLook
close = Fechar
binary-preview-hint = Arquivo binário ou muito grande — formato sem prévia direta.
git-status-label = Status Git

# Rodapé e transferências
selected-none = Selecionado: Nenhum
free-of = livres de
clipboard-copy-tag = Copiados
clipboard-cut-tag = Recortados
transferring = Transferindo...

# Operações de arquivo e modais
new-folder = Nova Pasta
new-file = Novo Arquivo
rename = Renomear (F2)
copy = Copiar (Ctrl+C)
cut = Recortar (Ctrl+X)
paste = Colar (Ctrl+V)
move-to-trash = Enviar para Lixeira (Del)
delete-permanent = Excluir Permanentemente (Shift+Del)
confirm = Confirmar
cancel = Cancelar
create = Criar
name-label = Nome
invalid-name = Nome inválido.
confirm-trash-msg = Deseja enviar os itens selecionados para a lixeira?
confirm-delete-msg = Esta ação remove permanentemente do disco e não pode ser desfeita.
conflict-title = Conflito no Destino
conflict-msg = Alguns itens já existem no destino. Como deseja continuar?
conflict-rename = Manter ambos (Renomear)
conflict-overwrite = Sobrescrever
conflict-skip = Pular existentes
open-item = Abrir
open-in-new-tab = Abrir em nova aba
copy-path = Copiar caminho
select-all = Selecionar tudo (Ctrl+A)
path-copied = Caminho copiado
properties = Propriedades

# Preferências
prefs-title = Preferências
theme-label = Tema
lang-label = Idioma
hero-card-label = Exibir cartão resumo da pasta
decorations-label = Barra de janela
default-view-label = Visualização padrão
view-grid = Grade
view-list = Lista
window-bar-native = Barra Nativa
window-bar-csd = CSD Integrado

# Janela de Propriedades
location-label = Localização
copy-sha256 = Copiar SHA-256
hash-copied = SHA-256 copiado
properties-header-prefix = { "Informações: " }
prop-tab-general = Geral
prop-tab-security = Segurança
prop-tab-history = Histórico
prop-tab-metadata = Metadados
prop-tab-permissions = Permissões
prop-kind-label = Tipo
prop-system-mime-label = Sistema / MIME
prop-accessed-label = Último acesso
prop-quick-props = Propriedades Rápidas
prop-readonly = Somente leitura
prop-hidden = Oculto
prop-advanced-settings = Configurações Avançadas
prop-confirm = Confirmar
prop-apply = Aplicar
prop-open-default = Abrir padrão
prop-owner = Proprietário
prop-group = Grupo
prop-others = Outros
prop-octal-mode = Modo octal
prop-symlink-target = Alvo do symlink
prop-archive-notice = Somente leitura (dentro de arquivo compactado)
prop-applied-notice = Propriedades aplicadas.
prop-pending-changes = Alterações pendentes
prop-calculating = Calculando...
prop-sha-na-folders = Não aplicável a pastas
prop-sha-computing = Calculando hash SHA-256...
yes-label = Sim
no-label = Não
ext-label = Extensão
perm-read = Leitura
perm-write = Escrita
perm-execute = Execução

# Tipos de entrada
kind-folder = Pasta de Arquivos
kind-symlink-dir = Link simbólico (Pasta)
kind-symlink-file = Link simbólico (Arquivo)
kind-special = Item Especial
kind-file = Arquivo
kind-file-ext = Arquivo (.{ $ext })

# Formatação pluralizada e parametrizada
items-count = { $count ->
    [one] 1 item
   *[other] { $count } itens
}
folders-count = { $count ->
    [one] 1 pasta
   *[other] { $count } pastas
}
files-count = { $count ->
    [one] 1 arquivo
   *[other] { $count } arquivos
}
selected-multi = { $count ->
    [one] 1 item ({ $size })
   *[other] { $count } itens ({ $size })
}
exact-bytes = ({ $bytes } bytes)
age-now = agora
age-minutes = há { $m }m
age-hours = há { $h }h
age-days = há { $d }d
ts-today = Hoje, { $time }
ts-yesterday = Ontem, { $time }

# Abas de Preferências e Sobre
prefs-tab-general = Geral
about-tab = Sobre
about-subtitle = Gerenciador de arquivos nativo de alta performance para Linux e Windows
about-version-label = Versão
about-developer-label = Desenvolvedor
about-developer-value = em desenvolvimento
about-stage-label = Estágio
about-stage-alpha = Alpha — Prévia de Desenvolvimento
about-license-label = Licença
about-stack-label = Motor / Stack
about-copy-info = Copiar informações
about-info-copied = Informações copiadas

# Propriedades de pasta (conteúdo e espaço em disco)
prop-contents-label = Conteúdo
prop-free-space-label = Espaço livre
hidden-items-count = { $count ->
    [one] 1 oculto
   *[other] { $count } ocultos
}

# Dicas de ferramentas (Tooltips)
tip-new-tab = Abrir nova aba (Ctrl+T)
tip-close-tab = Fechar aba (Ctrl+W)
tip-language = Alterar idioma da interface
tip-theme = Alternar tema claro/escuro
tip-back = Voltar no histórico (Alt+←)
tip-forward = Avançar no histórico (Alt+→)
tip-up = Subir para a pasta pai (Alt+↑)
tip-home = Ir para a pasta pessoal
tip-edit-path = Editar caminho diretamente (Ctrl+L)
tip-deep-search = Buscar recursivamente em subpastas (Ctrl+Shift+F)
tip-view-grid = Visualização em Grade
tip-view-list = Visualização em Tabela de Detalhes
tip-new-item = Criar nova pasta ou arquivo
tip-group-folders = Agrupar pastas antes dos arquivos no Grid
tip-zoom-out = Diminuir tamanho dos cards (Ctrl+-)
tip-zoom-reset = Restaurar tamanho padrão dos cards (Ctrl+0)
tip-zoom-in = Aumentar tamanho dos cards (Ctrl++)
tip-sort = Alternar critério de ordenação (Nome, Tamanho, Tipo, Modificado)
tip-sort-dir = Alternar ordem crescente ou decrescente
tip-filter = Filtrar itens por pasta ou extensão
tip-hidden = Mostrar ou ocultar arquivos ocultos (Ctrl+H)
tip-terminal = Abrir terminal neste diretório (Ctrl+Shift+T)
tip-favorite = Fixar ou remover diretório dos favoritos
tip-inspector = Mostrar ou ocultar painel Inspetor (Alt+I)
tip-preferences = Abrir Preferências (clique direito para menu rápido)
tip-toggle-sidebar = Recolher ou expandir barra lateral (Ctrl+B)

# Locais padrão (Nemo) e montagem de discos
music = Música
videos = Vídeos
public-dir = Público
templates = Modelos
filesystem-root = Sistema de Arquivos
mount-drive = Montar
unmount-drive = Desmontar
unmounted-label = Desmontado
refresh-drives = Atualizar unidades
tip-expand-home = Expandir ou recolher pastas de Início
open-as-root = Abrir como root
root-mode-badge = Modo Root (Privilégios elevados)
grid-center-label = Alinhamento dos itens no Grid
grid-align-left = Esquerda para direita
grid-align-center = Centralizado
tip-grid-alignment = Alternar alinhamento da grade (Esquerda para direita ou Centralizado)
tip-system-icons = Alternar entre ícones padrão e ícones do sistema
perm-allow-exec = Autorizar execução como programa
open-with = Abrir com...
prop-tab-open-with = Abrir com
open-with-prompt = Selecione um aplicativo na lista para abrir este arquivo e outros do mesmo tipo:
open-with-default-app = Aplicativo padrão
open-with-recommended-apps = Aplicativos recomendados
open-with-other-apps = Outros aplicativos
open-with-custom-hint = Você também pode digitar ou selecionar um comando ou executável personalizado:
open-with-custom-placeholder = Ex.: mpv --no-terminal ou /caminho/programa
open-with-add-to-list = Adicionar à lista
open-with-set-default = Definir como padrão
open-with-reset-default = Restaurar padrões do sistema
open-with-open-once = Abrir agora
open-with-loading = Buscando aplicativos instalados...
open-with-empty = Nenhum aplicativo compatível encontrado.
open-with-saved = Aplicativo padrão atualizado.
open-with-reset-done = Padrão do sistema restaurado.
open-with-pick-exec = Selecionar arquivo executável

# Renomeação em Lote
br-title = Renomear em Lote
br-menu-item = Renomear em Lote...
br-clear-rules = Limpar regras
br-rules-header = O que você quer fazer?
br-preview-header = Preview em Tempo Real
br-col-status = Status
br-col-original = Nome Atual
br-col-new = Novo Nome
br-unchanged = (sem mudança)
br-skipped-folder = (pasta ignorada)
br-conflict-internal = Conflito: mesmo nome final
br-conflict-external = Conflito: já existe no disco
br-invalid-name = Nome inválido
br-replace-title = Procurar e Substituir
br-find-label = Procurar
br-replace-label = Substituir por
br-where-label = Onde aplicar
br-scope-anywhere = Em qualquer lugar
br-scope-start-name = Só no início do nome
br-scope-end-name = Só no fim do nome
br-scope-word-start = Só início da palavra
br-scope-word-end = Só fim da palavra
br-scope-whole-word = Palavra inteira
br-match-case = Diferenciar MAIÚSC./minúsc.
br-first-only = Só a 1ª ocorrência
br-advanced-regex = Opções avançadas (Regex)
br-use-regex = Usar expressão regular (Regex)
br-remove-title = Remover Texto / Caracteres
br-remove-first = Primeiros caract.
br-remove-last = Últimos caract.
br-remove-text-label = Remover trecho
br-remove-digits = Números (0-9)
br-remove-symbols = Símbolos
br-collapse-spaces = Espaços extras
br-insert-title = Inserir Texto
br-insert-text-label = Texto a inserir
br-insert-pos-label = Posição
br-insert-start = No início do nome
br-insert-end = No fim do nome
br-insert-offset-start = Na posição (do início)
br-insert-offset-end = Na posição (do fim)
br-case-title = Maiúsculas / Minúsculas
br-case-keep = Sem alteração
br-case-upper = TUDO MAIÚSCULA
br-case-lower = tudo minúscula
br-case-title-words = Primeira Letra De Cada Palavra
br-case-sentence = Só a 1ª letra maiúscula
br-number-title = Numeração Sequencial
br-number-append = No fim do nome
br-number-prepend = No início do nome
br-number-replace = Substituir todo o nome
br-number-start = Início
br-number-step = Passo
br-number-digits = Dígitos
br-number-sep = Separador
br-sort-label = Ordem
br-sort-selection = Ordem selecionada
br-sort-name-asc = Nome (A-Z)
br-sort-name-desc = Nome (Z-A)
br-sort-date-desc = Mais recentes
br-sort-date-asc = Mais antigos
br-sort-size-asc = Menores primeiro
br-sort-size-desc = Maiores primeiro
br-media-title = Dados da Foto / Música (EXIF e ID3)
br-media-exif-ymd = Data da foto (AAAA-MM-DD)
br-media-exif-datetime = Data e hora da foto
br-media-exif-camera = Modelo da câmera
br-media-id3-track = Nº da faixa
br-media-id3-artist = Artista
br-media-id3-album = Álbum
br-media-id3-title = Título da música
br-media-id3-year = Ano
br-media-loading = Lendo tags EXIF/ID3...
br-ext-title = Mudar Extensão
br-ext-keep = Manter original
br-ext-lower = minúsculas (.jpg)
br-ext-upper = MAIÚSCULAS (.JPG)
br-ext-replace = Trocar para
br-ext-remove = Remover extensão
br-include-folders = Incluir pastas
br-apply = Aplicar
br-applying = Renomeando...
br-undo-last = Desfazer última renomeação
br-readonly-error = Renomeação em lote não é suportada na Lixeira ou em arquivos compactados.

# Extração e compactação de arquivos
arch-extract-here = Extrair aqui
arch-extract-subdir = Extrair em subpasta
arch-compress-as = Compactar como...
arch-add-7z = Adicionar ao arquivo .7z
arch-modal-title = Compactar como...
arch-output-name = Nome do arquivo
arch-destination = Pasta de destino
arch-format-label = Formato / Extensão
arch-level-label = Nível de compressão
arch-format-7z = .7z (7-Zip — Alta compressão)
arch-format-zip = .zip (ZIP — Universal)
arch-format-tar = .tar (TAR — Pacote sem compressão)
arch-format-targz = .tar.gz (TAR.GZ — Comprimido com Gzip)
arch-level-store = Armazenar (sem compressão)
arch-level-min = Mínimo (rápido)
arch-level-normal = Normal (equilibrado)
arch-level-max = Máximo (maior compressão)
arch-tar-store-hint = O formato TAR apenas empacota os arquivos (somente Armazenar).
arch-conflict-label = Se já existir
arch-conflict-rename = Renomear automaticamente
arch-conflict-overwrite = Sobrescrever
arch-conflict-skip = Cancelar se existir
arch-start-compress = Compactar
arch-minimize = Minimizar
arch-restore = Restaurar
arch-compressing = Compactando
arch-extracting = Extraindo
arch-done-compress = Compactação concluída
arch-done-extract = Extração concluída
arch-cancelled = Operação cancelada
arch-password-title = Arquivo protegido por senha
arch-password-hint = Digite a senha para extrair este arquivo.
arch-password-placeholder = Senha...
arch-password-wrong = Senha incorreta. Tente novamente.
arch-password-submit = Extrair
arch-password-toggle = Mostrar ou ocultar senha

# Buscador de arquivos duplicados
dup-menu-action = Buscar arquivos duplicados...
dup-modal-title = Buscar Arquivos Duplicados
dup-readonly-error = Busca de duplicados não é suportada na Lixeira ou em arquivos compactados.
dup-confirm-level-label = Nível de confirmação da duplicidade
dup-level-quick = 1. Rápido — Tamanho + MD5 Parcial (Triagem)
dup-level-standard = 2. Padrão — Tamanho + MD5 Parcial + MD5 Completo
dup-level-sha256 = 3. Seguro — MD5 Completo + Confirmação SHA-256
dup-level-byte = 4. Máximo — MD5 Completo + Comparação Byte a Byte
dup-min-size-label = Tamanho mínimo
dup-min-size-any = Qualquer (≥ 1 B)
dup-min-size-1kb = ≥ 1 KB
dup-min-size-1mb = ≥ 1 MB
dup-min-size-10mb = ≥ 10 MB
dup-start-scan = Iniciar Busca
dup-rescan = Reescanear
dup-stop-scan = Parar
dup-opt-subfolders = Incluir subpastas
dup-opt-hidden = Incluir ocultos
dup-opt-ignore-empty = Ignorar vazios (0 B)
dup-opt-ignore-hardlinks = Ignorar hardlinks
dup-quick-warning = Modo Rápido compara apenas início, meio e fim dos arquivos. Para exclusão definitiva, recomenda-se Padrão, Seguro ou Máximo.
dup-quick-select-label = Seleção rápida:
dup-keep-oldest = Manter mais antigo
dup-keep-newest = Manter mais recente
dup-keep-shortest = Caminho mais curto
dup-clear-selection = Limpar
dup-badge-oldest = Mais antigo
dup-badge-hardlink = Hardlink
dup-verify-partial = MD5 Parcial (aprox.)
dup-verify-md5 = MD5 Completo ✓
dup-verify-sha256 = MD5 + SHA-256 ✓
dup-verify-byte = Byte-a-Byte 100% ✓
dup-stage-walking = Varrendo pasta
dup-stage-partial = MD5 Parcial
dup-stage-full = MD5 Completo
dup-stage-sha256 = MD5 + SHA-256
dup-stage-byte = MD5 + Byte-a-Byte
dup-all-selected-warning = Atenção: você marcou TODAS as cópias de pelo menos 1 grupo! Desmarque 1 exemplar para não perder o arquivo original.
dup-delete-permanent = Excluir permanentemente
dup-move-to-trash = Mover para Lixeira
dup-confirm-perm-delete = Confirmar exclusão permanente?
dup-empty-idle = Escolha o nível de confirmação e clique em Iniciar Busca para localizar arquivos duplicados.
dup-empty-none = Nenhum arquivo duplicado encontrado com os filtros atuais.
dup-group-prefix = Grupo
dup-copies-word = cópias
dup-each-word = cada
dup-selected-word = marcados
dup-groups-found = grupos encontrados
dup-reclaimable = recuperáveis
dup-footer-prefix = Duplicados
dup-cancelled = Busca de duplicados cancelada
dup-stale-skipped = Alguns arquivos mudaram ou sumiram no disco e foram ignorados por segurança.
dup-last-copy-protected = Exclusão bloqueada em grupo(s) cuja última cópia não marcada não existe mais em disco.
dup-delete-done = Duplicatas removidas com segurança
ctx-send-to = Enviar para...
ctx-shortcut-desktop = Criar atalho na Área de Trabalho
ctx-shortcut-here-unix = Criar link simbólico aqui
ctx-shortcut-here-win = Criar atalho aqui
shortcut-desktop-missing = Pasta da Área de Trabalho não encontrada.
filter-ext-placeholder = ex: .exe, .zip...
shortcuts-created =
    { $count ->
        [one] Atalho criado com sucesso.
       *[other] { $count } atalhos criados com sucesso.
    }
undo-action = Desfazer (Ctrl+Z)
undo-trash-action = Desfazer exclusão (Ctrl+Z)
moved-to-trash-undo-hint = Enviado para a Lixeira (Ctrl+Z para desfazer)
undo-nothing-in-trash = Item não encontrado na Lixeira para restaurar.
undo-rename-done = Renomeação desfeita.
undo-restored-count =
    { $count ->
        [one] Restaurado da Lixeira.
       *[other] { $count } itens restaurados da Lixeira.
    }
mount-disk-image = Montar imagem de disco
unmount-disk-image = Desmontar imagem
eject-safely = Remover com segurança
image-mounted = Imagem montada com sucesso
drive-ejected = Unidade removida com segurança
drive-free-label = livres
drive-click-to-mount = Clique para montar
tip-split-view = Dividir tela (F3)
close-split-pane = Fechar painel dividido (F3)
drag-move-prefix = Mover
drag-copy-prefix = Copiar
drag-ctrl-copy-hint = Ctrl = Copiar
update-available-banner = Nova versão estável disponível:
update-download-btn = Baixar atualização
update-ignore-btn = Ignorar esta versão
update-check-btn = Verificar atualizações
update-checking = Verificando atualizações...
update-up-to-date = Você já está na versão estável mais recente.
update-check-failed = Não foi possível verificar atualizações agora.
update-auto-check-label = Verificar atualizações estáveis ao iniciar
update-open-releases = Abrir página da release
tip-update-available = Nova versão estável disponível — clique para abrir a página da release
recent-files = Recentes
prefs-recent-files-label = Mostrar arquivos recentes
clear-recent-files = Limpar recentes
remove-from-recent = Remover dos recentes
open-file-location = Abrir local do arquivo
open-target-location = Abrir local do destino
prefs-startup-location-label = Pasta inicial ao abrir
startup-home = Pasta do usuário
startup-last-visited = Última pasta acessada
ntfs-object-name = Nome do objeto:
ntfs-principals-label = Nomes de grupo ou de usuário:
ntfs-edit-hint = Para alterar permissões, clique em Editar.
ntfs-edit-active-hint = Marque Permitir ou Negar individualmente (ou use Controle total).
ntfs-edit-btn = Editar...
ntfs-done-edit-btn = Concluir edição
ntfs-add-btn = Adicionar...
ntfs-remove-btn = Remover
ntfs-permissions-for = Permissões para
ntfs-allow-col = Permitir
ntfs-deny-col = Negar
ntfs-full-control = Controle total
ntfs-list-folder = Listar conteúdo da pasta
ntfs-delete-modify = Excluir / Modificar
ntfs-inherited-badge = Herdado
ntfs-explicit-badge = Explícito
ntfs-owner-label = Proprietário
ntfs-inheritance-label = Herança NTFS
ntfs-inheritance-on = Ativada (herdando do pai)
ntfs-inheritance-off = Desativada (explícitas)
ntfs-add-placeholder = Ex.: Usuários, Todos, Administradores...
ntfs-loading = Carregando permissões NTFS...
posix-owner-group-header = Proprietário e Grupo (UID / GID)
posix-octal-symbolic-header = Modo Octal e Simbólico
posix-presets-label = Presets:
posix-preset-default = Padrão
posix-preset-exec = Executável
posix-preset-private = Privado
posix-preset-readonly = Somente leitura
posix-preset-group = Grupo
posix-preset-shared = Compartilhada
posix-scope-col = Escopo
posix-summary-col = Resumo
posix-others-sub = todos os usuários
posix-special-bits-header = Bits Especiais POSIX
posix-quick-action-header = Atalho Rápido
posix-recursive-header = Escopo em Subpastas
posix-apply-recursive = Aplicar recursivamente ao conteúdo

analyze-space = Analisar espaço
tip-analyze-space = Analisar uso de espaço em disco e arquivos grandes
disk-scan-eyebrow = Analisando
disk-scan-title = Procurando por arquivos grandes...
disk-scan-current-item = Item atual
disk-scan-files-checked = Arquivos verificados
disk-scan-data-checked = Dados verificados
disk-scan-elapsed = Tempo decorrido
disk-scan-hint = Verificando o local selecionado; o tempo depende da quantidade de arquivos
disk-results-title = Análise de Espaço em Disco
disk-root-crumb = Raiz selecionada
disk-treemap-title = Mapa proporcional (clique para explorar)
disk-top-consumers = Maiores consumidores neste nível
disk-by-type = Por tipo de arquivo
disk-mark-cleanup = Marque arquivos ou pastas para apagar
disk-reanalyze = Reanalisar
disk-clear-selection = Limpar seleção
disk-reclaim-suffix = serão liberados
disk-mode-files = Maiores arquivos
disk-mode-folders = Pastas
disk-cat-iso = ISOs / Discos
disk-cat-build = Builds / Cache
disk-cat-archives = Compactados
disk-cat-media = Mídia
disk-cat-docs = Documentos
disk-cat-other = Outros

search-name-label = Nome / Caminho
search-containing-label = Pesquisar contendo...
search-containing-placeholder = Texto ou regex dentro dos arquivos (pula binários)...
search-size-label = Tamanho
search-date-label = Modificado
search-type-label = Tipo
search-hidden-label = Ocultos
search-streaming-hint = Streaming sem índice • Pula binários
search-any = Qualquer
search-today = Hoje
search-last-7d = 7 dias
search-last-30d = 30 dias
search-type-code = Código
search-type-docs = Documentos
search-type-images = Imagens
search-type-audio = Áudio
search-type-video = Vídeo
search-type-archives = Arquivos

cmd-palette-title = Paleta de Comandos
cmd-palette-placeholder = Digite uma ação, categoria ou atalho...
cmd-palette-empty = Nenhum comando encontrado.
cmd-palette-hint = ↑↓ navegar • Enter executar • Esc fechar
cmd-cat-nav = Navegação
cmd-cat-search = Busca
cmd-cat-files = Arquivos
cmd-cat-view = Visualização
cmd-cat-tools = Ferramentas
cmd-focus-quick-filter = Focar filtro rápido da aba
cmd-search-containing = Pesquisar contendo texto nos arquivos...
cmd-clear-filters = Limpar filtros da aba
cmd-check-updates = Verificar atualizações
cmd-toggle-theme = Alternar tema claro/escuro
cmd-view-grid = Visualização em Grade
cmd-view-list = Visualização em Lista

quick-jump-title = Ir para pasta (Quick Jump)
quick-jump-placeholder = Digite o nome ou caminho da pasta...
quick-jump-empty = Nenhuma pasta encontrada.
quick-jump-hint = ↑↓ navegar • Enter abrir • Ctrl+Enter nova aba • Esc fechar
quick-jump-src-current = Atual
quick-jump-src-favorite = Favorito
quick-jump-src-recent = Recente
quick-jump-src-subfolder = Subpasta
quick-jump-new-tab = Nova aba
