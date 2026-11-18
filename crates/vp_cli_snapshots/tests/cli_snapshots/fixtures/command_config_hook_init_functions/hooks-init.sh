export HOOK_INIT_LOADED=yes

cd() {
  command cd "$@" && printf 'Now using node v24.0.0\n'
}

pwd() {
  command pwd "$@"
  printf 'pwd wrapper output\n'
}
