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
view-columns = Colunas
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
tip-view-columns = Visualização em Colunas (Miller)
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
