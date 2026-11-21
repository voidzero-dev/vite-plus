export HOOK_INIT_LOADED=yes
# Later reads of PWD must not change which package supplies hook commands.
PWD="$PWD/decoy"

cd() {
  command cd "$@" && printf 'Now using node v24.0.0\n'
}

pwd() {
  command pwd "$@"
  printf 'pwd wrapper output\n'
}
