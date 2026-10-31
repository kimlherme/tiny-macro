#!/usr/bin/env bash
set -Eeuo pipefail

# Instalação para o usuário atual; execute como usuário normal, não com sudo.
if [[ ${1:-} == --help ]]; then
  printf 'Uso: ./install.sh\nInstalador interativo para Omarchy e Arch Linux.\n'
  exit 0
fi
if [[ $# != 0 || ! -t 0 || ! -t 1 ]]; then
  printf 'Execute ./install.sh em um terminal interativo. Use --help para ajuda.\n' >&2
  exit 2
fi
if (( EUID == 0 )); then
  printf 'Execute como usuário normal: ./install.sh (o sudo será solicitado quando necessário).\n' >&2
  exit 1
fi
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
user_name=$(id -un)
use_whiptail=false
command -v whiptail >/dev/null 2>&1 && use_whiptail=true
info() {
  if $use_whiptail; then whiptail --title 'Tiny Macro Linux' --msgbox "$1" 13 76; else printf '\n%s\n' "$1"; fi
}
confirm() {
  if $use_whiptail; then whiptail --title 'Tiny Macro Linux' --yesno "$1" 15 76; else
    local answer
    printf '\n%s [s/N] ' "$1"
    read -r answer
    [[ $answer == [sS] || $answer == [sS][iI][mM] ]]
  fi
}
trap 'printf "\nInstalação interrompida. Confira a mensagem acima; alterações já feitas não são revertidas automaticamente.\n" >&2' ERR
info 'Bem-vindo! Este instalador compila o Tiny Macro Linux, cria um atalho e oferece configurar as permissões. Funciona no Omarchy/Arch Linux.'
if ! command -v pacman >/dev/null 2>&1; then
  info 'Esta versão do instalador oferece suporte somente ao Arch Linux e derivados.'
  exit 1
fi
if confirm 'Instalar ou conferir os pacotes rust, gtk4 e pkgconf com pacman? Será solicitada sua senha pelo sudo.'; then
  sudo pacman -S --needed rust gtk4 pkgconf
else
  for cmd in cargo rustc pkg-config; do
    if ! command -v "$cmd" >/dev/null 2>&1; then info "Dependência ausente: $cmd. Instale as dependências e tente novamente."; exit 1; fi
  done
  if ! pkg-config --exists gtk4; then info 'GTK 4 não encontrado. Instale o pacote gtk4 e tente novamente.'; exit 1; fi
fi
printf '\nCompilando Tiny Macro Linux...\n'
cargo build --release --locked --manifest-path "$project_dir/Cargo.toml"
install_dir="${XDG_DATA_HOME:-$HOME/.local/share}/tiny-macro-linux"
applications_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
mkdir -p "$install_dir" "$applications_dir"
install -m 755 "$project_dir/target/release/tiny-macro-linux" "$install_dir/tiny-macro-linux"
# Exec usa as regras de aspas do Desktop Entry, não as do shell.
exec_path="$install_dir/tiny-macro-linux"
escaped_path=${exec_path//\\/\\\\}
escaped_path=${escaped_path//\"/\\\"}
escaped_path=${escaped_path//\$/\\$}
escaped_path=${escaped_path//\`/\\\`}
escaped_path=${escaped_path//%/%%}
cat > "$applications_dir/tiny-macro-linux.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Tiny Macro Linux
Comment=Gravar e reproduzir teclado e mouse
Exec="$escaped_path"
Icon=input-keyboard
Terminal=false
Categories=Utility;
Keywords=macro;tinytask;teclado;mouse;
StartupWMClass=local.tinymacro.Linux
DESKTOP
if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database "$applications_dir"; fi
permission_message='Configurar acesso permanente a teclado e mouse? O grupo input permite que TODOS os programas da sua conta leiam eventos de entrada e simulem teclas e cliques. Isto pode expor senhas. A configuração exige sudo e normalmente requer sair e entrar novamente. Se recusar, o aplicativo será instalado, mas gravação e reprodução podem não funcionar.'
if confirm "$permission_message"; then
  module_file=/etc/modules-load.d/tiny-macro-uinput.conf
  rule_file=/etc/udev/rules.d/70-tiny-macro-uinput.rules
  expected_rule='KERNEL=="uinput", GROUP="input", MODE="0660"'
  if [[ -e $module_file ]] && [[ $(cat "$module_file") != uinput ]]; then
    info "O arquivo $module_file já existe com outro conteúdo. Não será alterado; configure as permissões manualmente."; exit 1
  fi
  if [[ -e $rule_file ]] && [[ $(cat "$rule_file") != "$expected_rule" ]]; then
    info "O arquivo $rule_file já existe com outro conteúdo. Não será alterado; configure as permissões manualmente."; exit 1
  fi
  sudo usermod -aG input "$user_name"
  printf '%s\n' uinput | sudo tee "$module_file" >/dev/null
  printf '%s\n' "$expected_rule" | sudo tee "$rule_file" >/dev/null
  sudo modprobe uinput
  sudo udevadm control --reload-rules
  sudo udevadm trigger --action=change /dev/uinput
  info 'Instalação concluída. Saia da sessão e entre novamente para aplicar o grupo input. Procure por Tiny Macro Linux no menu.'
else
  info 'Aplicativo instalado sem alterar permissões. Procure por Tiny Macro Linux no menu. Para gravar e reproduzir, configure o acesso aos dispositivos conforme o README.'
fi
