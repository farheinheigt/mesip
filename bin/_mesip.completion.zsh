#compdef mesip
_mesip_completion() {
  _arguments -s \
    '(-h --help)'{-h,--help}'[Afficher l aide]' \
    '--no-public[Ne pas interroger les services externes pour l IP publique]' \
    '--timeout=[Timeout reseau (secondes)]:secondes:' \
    '--no-color[Desactiver les couleurs ANSI]' \
    '--completion[Generer la completion shell]:shell:(zsh)'
}

compdef _mesip_completion mesip
